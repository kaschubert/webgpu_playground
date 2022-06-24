use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
    dpi::PhysicalPosition,
};
use futures::task::SpawnExt;

mod ui;
use ui::controls::Controls;
use iced::{Sandbox};
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Clipboard, Debug, Size};
mod util;
mod wasm;

mod state;
use state::state::State;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn start() {
    #[cfg(target_arch = "wasm32")]
    let canvas_element = {
        console_log::init_with_level(log::Level::Debug)
            .expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.get_element_by_id("iced_canvas"))
            .and_then(|element| element.dyn_into::<HtmlCanvasElement>().ok())
            .expect("Canvas with id `iced_canvas` is missing")
    };
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let event_loop = EventLoop::new();

    #[cfg(target_arch = "wasm32")]
    let window = winit::window::WindowBuilder::new()
        .with_canvas(Some(canvas_element))
        .build(&event_loop)
        .expect("Failed to build winit window");

    #[cfg(not(target_arch = "wasm32"))]
    let window = WindowBuilder::new()
        .with_title(env!("CARGO_PKG_NAME"))
        .build(&event_loop)
        .unwrap();

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window).await.unwrap();

    let mut cursor_position = PhysicalPosition::new(-1.0, -1.0);
    let mut modifiers = ModifiersState::default();
    let mut clipboard = Clipboard::connect(&window);

    let controls = Controls::new();

    // Initialize iced
    let mut debug = Debug::new();
    let mut renderer =
        Renderer::new(Backend::new(&mut state.device, Settings::default(), state.format));

    let mut iced_state = program::State::new(
        controls,
        state.viewport.logical_size(),
        &mut renderer,
        &mut debug,
    );

    let mut resized = false;
    // Initialize staging belt and local pool
    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();


    event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    match event {
                        WindowEvent::CursorMoved { position, .. } => {
                            cursor_position = *position;
                        }
                        WindowEvent::ModifiersChanged(new_modifiers) => {
                            modifiers = *new_modifiers;
                        }
                        WindowEvent::Resized(physical_size) => {
                            resized = state.resize(*physical_size, window.scale_factor());
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            resized = state.resize(**new_inner_size, window.scale_factor());
                        }
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                    // Map window event to iced event
                    if let Some(iced_event) = iced_winit::conversion::window_event(
                        &event,
                        window.scale_factor(),
                        modifiers,
                    ) {
                        iced_state.queue_event(iced_event);
                    }
                    state.input(event);
                },
                Event::MainEventsCleared => {
                    // If there are events pending
                    if !iced_state.is_queue_empty() {
                        // We update iced
                        let _ = iced_state.update(
                    state.viewport.logical_size(),
                            conversion::cursor_position(
                                cursor_position,
                                state.viewport.scale_factor(),
                            ),
                            &mut renderer,
                            &mut clipboard,
                            &mut debug,
                        );
                    }
                
                    if resized {
                        let size = window.inner_size();

                        state.surface.configure(
                            &state.device,
                            &wgpu::SurfaceConfiguration {
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                format: state.format,
                                width: size.width,
                                height: size.height,
                                present_mode: wgpu::PresentMode::Mailbox,
                            },
                        );

                        resized = false;
                    }

                    match state.surface.get_current_texture() {
                        Ok(frame) => {
                            let mut encoder = state.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor { label: None },
                            );
                            let program = iced_state.program();
                            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                            {
                                // We clear the frame
                                state.clear(
                                    &view,
                                    &mut encoder,
                                    program.background_color(),
                                );
                            }

                            // Draw the scene
                            state.update();
                            match state.render(&mut encoder, &frame) {
                                Ok(_) => {}
                                // Reconfigure the surface if lost
                                Err(wgpu::SurfaceError::Lost) => resized = state.resize(state.physical_size, window.scale_factor()),
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => eprintln!("{:?}", e),
                            }

                            // And then iced on top
                            renderer.with_primitives(|backend, primitive| {
                                backend.present(
                                    &mut state.device,
                                    &mut staging_belt,
                                    &mut encoder,
                                    &view,
                                    primitive,
                                    &state.viewport,
                                    &debug.overlay(),
                                );
                            });

                            // Then we submit the work
                            staging_belt.finish();
                            state.queue.submit(Some(encoder.finish()));
                            frame.present();

                            // Update the mouse cursor
                            window.set_cursor_icon(
                                iced_winit::conversion::mouse_interaction(
                                    iced_state.mouse_interaction(),
                                ),
                            );

                            // And recall staging buffers
                            local_pool
                                .spawner()
                                .spawn(staging_belt.recall())
                                .expect("Recall staging buffers");

                            local_pool.run_until_stalled();
                        }
                        Err(error) => match error {
                            wgpu::SurfaceError::OutOfMemory => {
                                panic!("Swapchain error: {}. Rendering cannot continue.", error)
                            }
                            _ => {
                                // Try rendering again next frame.
                                window.request_redraw();
                            }
                        },
                    }
                }
            _ => {}
        }
    });
}