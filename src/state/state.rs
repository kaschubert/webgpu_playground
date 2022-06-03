use crate::state::render_components::color_renderpass::ColorPass;
use crate::state::render_components::depth_renderpass::DepthPass;


use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
};

use wgpu::*;
use wgpu::util::DeviceExt;

use cgmath::*;

use bytemuck::Zeroable;

pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    color_pass: ColorPass,
    depth_pass: DepthPass,
}

impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width, // 0 can crash the app
            height: size.height, // 0 can crash the app
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let color_pass = ColorPass::new(&device, &config, &queue);
        let depth_pass = DepthPass::new(&device, &config);

        Ok (Self {
            surface,
            device,
            queue,
            config,
            size,
            color_pass,
            depth_pass,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_pass.resize(&self.device, &self.config);
            self.color_pass.resize(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.color_pass.input(event)
    }

    pub fn update(&mut self) {
        self.color_pass.update(&self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        self.color_pass.render(&view, &mut encoder, &self.depth_pass.texture);
        self.depth_pass.render(&view, &mut encoder);

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}
