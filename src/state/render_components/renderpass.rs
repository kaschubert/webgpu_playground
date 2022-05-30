use wgpu::Device;
use wgpu::RenderPass;
use wgpu::CommandEncoder;
use super::super::state::State;

pub fn create_render_pass(name: & str, render_state: & State) -> CommandEncoder {
    let output = render_state.surface.get_current_texture().unwrap();
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = render_state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some(name),
    });

    {
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(name),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(render_state.clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &render_state.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
        });


        renderpass.set_pipeline(&render_state.render_pipeline_texture);

        if render_state.space_state_on {
            renderpass.set_bind_group(0, &render_state.diffuse_bind_group_2,&[]);    
        } else {
            renderpass.set_bind_group(0, &render_state.diffuse_bind_group,&[]);    
        }
        renderpass.set_bind_group(1, &render_state.camera_resources.camera_bind_group, &[]);

        renderpass.set_vertex_buffer(0, render_state.vertex_buffer.slice(..));
        renderpass.set_vertex_buffer(1, render_state.instance_buffer.slice(..));
        renderpass.set_index_buffer(render_state.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        renderpass.draw_indexed(0..render_state.num_indices, 0, 0..render_state.instances.len() as u32);
    }

    encoder

}



