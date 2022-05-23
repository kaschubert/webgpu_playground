use crate::state::render_components::vertex::Vertex;
use crate::state::render_components::texture::Texture;
use crate::state::render_components::instance::Instance;
use crate::state::render_components::instance::InstanceRaw;
use crate::state::render_components::instance::INSTANCE_DISPLACEMENT;
use crate::state::render_components::instance::NUM_INSTANCES_PER_ROW;
use crate::state::render_components::camera::*;
use crate::util::toggle_bool::BoolToggleExt;
use crate::util::math_funcs::quat_mul;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
};

use wgpu::*;
use wgpu::util::DeviceExt;

use cgmath::*;

use bytemuck::Zeroable;

const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732914], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], }, // E
];
const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
    /* padding */ 0,
];
const ROTATION_SPEED: f32 = 2.0 * std::f32::consts::PI / 180.0;

pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    render_pipeline_texture: wgpu::RenderPipeline,
    space_state_on: bool,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: Texture,
    diffuse_bind_group_2: wgpu::BindGroup,
    diffuse_texture_2: Texture,
    camera_resources: CameraResources,
    object_rotation: cgmath::Deg<f32>,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: Texture,
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

        let diffuse_bytes = include_bytes!("data/happy-tree.png");
        let diffuse_texture = Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();
        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");


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
    
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        let diffuse_bytes = include_bytes!("data/happy-tree-cartoon.png");
        let diffuse_texture_2 = Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree-cartoon.png").unwrap();

        let diffuse_bind_group_2 = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_2.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture_2.sampler),
                    }
                ],
                label: Some("diffuse_bind_group_2"),
            }
        );

        let clear_color = wgpu::Color::GREEN;

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader/shader.wgsl").into()),
        });

        let camera_resources = CameraResources::new(&config, &device)?;

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_resources.camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline_brown_tri = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline Textured Polygon"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main_textured_poly",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main_textured_poly",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: Option::None,
        });
        
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        let num_indices = INDICES.len() as u32;
        let object_rotation = cgmath::Deg(0.0);

        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                let position = cgmath::Vector3 { x: x as f32, y: 0.0, z: z as f32 } - INSTANCE_DISPLACEMENT;

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


        Ok (Self {
            surface,
            device,
            queue,
            config,
            size,
            clear_color,
            render_pipeline_texture: render_pipeline_brown_tri,
            space_state_on: false,
            vertex_buffer,
            index_buffer,
            num_indices,
            diffuse_bind_group,
            diffuse_texture,
            diffuse_bind_group_2,
            diffuse_texture_2,
            camera_resources,
            object_rotation,
            instances,
            instance_buffer,
            depth_texture,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_resources.camera_controller.process_events(event);

        match event {
            WindowEvent::CursorMoved { 
                position, ..
            } => {
                self.clear_color = wgpu::Color {
                    r: position.x as f64 / self.size.width as f64,
                    g: position.y as f64 / self.size.height as f64,
                    b: 1.0,
                    a: 1.0,
                };
                true
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

    pub fn update(&mut self) {
        self.object_rotation += cgmath::Deg(0.0);

        self.camera_resources.camera_controller.update_camera(&mut self.camera_resources.camera);
        self.camera_resources.camera_uniform.update_view_proj(&self.camera_resources.camera, self.object_rotation);
        self.queue.write_buffer(&self.camera_resources.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_resources.camera_uniform]));
    
        for instance in &mut self.instances {
            let amount = cgmath::Quaternion::from_angle_y(cgmath::Rad(ROTATION_SPEED));
            let current = instance.rotation;
            instance.rotation = quat_mul(amount, current);
        }

        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data),);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
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
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });


            render_pass.set_pipeline(&self.render_pipeline_texture);

            if self.space_state_on {
                render_pass.set_bind_group(0, &self.diffuse_bind_group_2,&[]);    
            } else {
                render_pass.set_bind_group(0, &self.diffuse_bind_group,&[]);    
            }
            render_pass.set_bind_group(1, &self.camera_resources.camera_bind_group, &[]);
            
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as u32);
        }
    
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}
