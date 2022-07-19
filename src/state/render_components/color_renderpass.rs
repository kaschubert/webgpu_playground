use wgpu::util::DeviceExt;
use cgmath::*;

use winit::{
    event::*,
};

use super::render_pipeline;

use super::texture::Texture;
use super::model::{Vertex, ModelVertex, Model, DrawModel, DrawLight};
use super::camera::CameraResources;
use super::instance::{Instance, InstanceRaw};
use super::light::LightResources;
use super::instance::NUM_INSTANCES_PER_ROW;

use crate::util::math_funcs::quat_mul;
use crate::util::toggle_bool::BoolToggleExt;
use crate::wasm::resources;

const ROTATION_SPEED: f32 = 2.0 * std::f32::consts::PI / 180.0;

pub struct ColorPass {
    pub clear_color: wgpu::Color,
    instance_buffer: wgpu::Buffer,
    instances: Vec<Instance>,
    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,
    pub camera_resources: CameraResources,
    light_resources: LightResources,
    space_state_on: bool,
    size: winit::dpi::PhysicalSize<u32>,
    object_rotation: Deg<f32>,
    model: Model,
}

impl ColorPass {
    pub async fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, queue: &wgpu::Queue) -> Self {    
        let clear_color = wgpu::Color::GREEN;
        let camera_resources = CameraResources::new(&config, &device).unwrap();
        let light_resources = LightResources::new(&device, [5.0, 5.0, 0.0], [1.0, 1.0, 1.0]);

        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        let render_pipeline = {
            let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_resources.camera_bind_group_layout,
                    &light_resources.light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Color Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shader/wgsl/shader.wgsl").into()),
            };

            render_pipeline::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_resources.camera_bind_group_layout, 
                    &light_resources.light_bind_group_layout
                ],
                push_constant_ranges: &[],
            });

            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shader/wgsl/light.wgsl").into()),
            };

            render_pipeline::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(Texture::DEPTH_FORMAT),
                &[ModelVertex::desc()],
                shader,
            )
        };

        
        const SPACE_BETWEEN: f32 = 3.0;

        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position = cgmath::Vector3 { x, y: 0.0, z };

                let rotation = if position.is_zero() {
                    // this is needed so an object at (0, 0, 0) won't get scaled to zero
                    // as Quaternions can effect scale if they're not created correctly
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                Instance {
                    position, rotation,
                }
            })
        }).collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX |wgpu::BufferUsages::COPY_DST,
            }
        );
        
        let size = winit::dpi::PhysicalSize::new(config.width, config.height);
        let object_rotation = cgmath::Deg(0.0f32);

        let model = resources::load_model(
            std::path::Path::new("models/cube/"),
            "cube.obj",
            &device,
            &queue,
            &texture_bind_group_layout,
        ).await.unwrap();

        Self {
            clear_color, 
            instance_buffer,
            instances,
            render_pipeline,
            light_render_pipeline,
            camera_resources,
            light_resources,
            space_state_on: false,
            size,
            object_rotation,
            model,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        self.camera_resources.camera.aspect = config.width as f32 / config.height as f32;
        self.size = winit::dpi::PhysicalSize::new(config.width, config.height);
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_resources.camera_controller.process_events(event);

        match event {
            WindowEvent::CursorMoved { 
                position, ..
            } => {
                false
            },
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Space),
                        ..
                    },
                ..
            } => {
                self.space_state_on.toggle();
                true
            },
            _ => false,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.object_rotation += cgmath::Deg(0.0);

        self.camera_resources.camera_controller.update_camera(&mut self.camera_resources.camera);
        self.camera_resources.camera_uniform.update_view_proj(&self.camera_resources.camera, self.object_rotation);
        queue.write_buffer(&self.camera_resources.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_resources.camera_uniform]));
    
        for instance in &mut self.instances {
            let amount = cgmath::Quaternion::from_angle_y(cgmath::Rad(ROTATION_SPEED));
            let current = instance.rotation;
            instance.rotation = quat_mul(amount, current);
        }

        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data),);
        self.light_resources.update(queue);
    }

    pub fn render(&self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder, depth_texture: &Texture) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_model_instanced(
            &self.model,
            0..self.instances.len() as u32,
            &self.camera_resources.camera_bind_group,
            &self.light_resources.light_bind_group
        );
        render_pass.set_pipeline(&self.light_render_pipeline);
        render_pass.draw_light_model(
            &self.model,
            &self.camera_resources.camera_bind_group,
            &self.light_resources.light_bind_group,
        );
    }
}