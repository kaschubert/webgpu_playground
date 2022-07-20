use crate::state::render_components::color_renderpass::ColorPass;
use crate::state::render_components::depth_renderpass::DepthPass;
use crate::state::render_components::texture::Texture;

use wgpu::SurfaceTexture;
use winit::{
    event::*,
    window::{Window},
};
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Clipboard, Debug, Size, Color};

pub struct State {
    pub surface: wgpu::Surface,
    pub format: wgpu::TextureFormat,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub physical_size: winit::dpi::PhysicalSize<u32>,
    pub viewport: Viewport,
    pub color_pass: ColorPass,
    //depth_pass: DepthPass,
    depth_texture: Texture,
}

impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> anyhow::Result<Self> {
        let physical_size = window.inner_size();
        let viewport = Viewport::with_physical_size(
            Size::new(physical_size.width, physical_size.height),
            window.scale_factor(),
        );

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        #[cfg(target_arch = "wasm32")]
        let default_backend = wgpu::Backends::GL;
        //let default_backend = wgpu::Backends::BROWSER_WEBGPU;
        
        #[cfg(not(target_arch = "wasm32"))]
        let default_backend = wgpu::Backends::PRIMARY;

        let backend = wgpu::util::backend_bits_from_env().unwrap_or(default_backend);
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(window) };
        
        let (format, (device, queue)) = futures::executor::block_on(async {
            let adapter = wgpu::util::initialize_adapter_from_env_or_default(
                &instance,
                backend,
                Some(&surface),
            )
            .await
            .expect("No suitable GPU adapters found on the system!");
    
            let adapter_features = adapter.features();
    
            #[cfg(target_arch = "wasm32")]
            let needed_limits = wgpu::Limits::downlevel_webgl2_defaults()
                .using_resolution(adapter.limits());
            #[cfg(not(target_arch = "wasm32"))]
            let needed_limits = wgpu::Limits::default();
    
            (
                surface
                    .get_preferred_format(&adapter)
                    .expect("Get preferred format"),
                adapter
                    .request_device(
                        &wgpu::DeviceDescriptor {
                            label: None,
                            features: adapter_features & wgpu::Features::default(),
                            limits: needed_limits,
                        },
                        None,
                    )
                    .await
                    .expect("Request device"),
            )
        });
        
        let config = wgpu::SurfaceConfiguration{
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: physical_size.width,
            height: physical_size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        surface.configure(
            &device,
            &config,
        );

        let color_pass = ColorPass::new(&device, &config, &queue).await;
        //let depth_pass = DepthPass::new(&device, &config);
        
        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        Ok (Self {
            surface,
            format,
            device,
            queue,
            config,
            physical_size,
            viewport,
            color_pass,
            //depth_pass,
            depth_texture,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, scale_factor: f64) -> bool{
        if new_size.width > 0 && new_size.height > 0 {
            self.physical_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            //self.depth_pass.resize(&self.device, &self.config);
            self.color_pass.resize(&self.device, &self.config);
            self.depth_texture = Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

            self.viewport = Viewport::with_physical_size(
                Size::new(new_size.width, new_size.height),
                scale_factor,
            );
            true
        }
        else {
            false
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.color_pass.input(event)
    }

    pub fn update(&mut self) {
        self.color_pass.update(&self.queue);
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &SurfaceTexture) -> Result<(), wgpu::SurfaceError> {
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.color_pass.render(&view, encoder, &self.depth_texture);
        //self.depth_pass.render(&view, encoder);

        Ok(())
    }
}
