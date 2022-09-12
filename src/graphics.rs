use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage,
        SecondaryAutoCommandBuffer,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    image::ImageViewAbstract,
    impl_vertex,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline, Pipeline, PipelineBindPoint},
    render_pass::Subpass,
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
};

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
                #version 450

                layout(location=0) in vec2 position;
                layout(location=1) in vec2 tex_coords;
                layout(location = 0) out vec2 f_tex_coords;

                void main() {
                    gl_Position =  vec4(position, 0.0, 1.0);
                    f_tex_coords = tex_coords;
                }
            "
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
                #version 450

                layout(location = 0) in vec2 v_tex_coords;
                layout(location = 0) out vec4 f_color;
                layout(set = 0, binding = 0) uniform sampler2D tex;

                void main() {
                    f_color = texture(tex, v_tex_coords);
                }
            "
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct TexturedVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}
impl_vertex!(TexturedVertex, position, tex_coords);

pub fn textured_quad(width: f32, height: f32) -> (Vec<TexturedVertex>, Vec<u32>) {
    (
        vec![
            TexturedVertex {
                position: [-(width / 2.0), -(height / 2.0)],
                tex_coords: [0.0, 1.0],
            },
            TexturedVertex {
                position: [-(width / 2.0), height / 2.0],
                tex_coords: [0.0, 0.0],
            },
            TexturedVertex {
                position: [width / 2.0, height / 2.0],
                tex_coords: [1.0, 0.0],
            },
            TexturedVertex {
                position: [width / 2.0, -(height / 2.0)],
                tex_coords: [1.0, 1.0],
            },
        ],
        vec![0, 2, 1, 0, 3, 2],
    )
}

pub struct Runner {
    queue: Arc<Queue>,
    subpass: Subpass,
    pipeline: Arc<GraphicsPipeline>,
    vertices: Arc<CpuAccessibleBuffer<[TexturedVertex]>>,
    indices: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl Runner {
    pub fn new(queue: Arc<Queue>, subpass: Subpass) -> Runner {
        let device = queue.device().clone();

        let (vertices, indices) = textured_quad(2.0, 2.0);
        let vertex_buffer = CpuAccessibleBuffer::<[TexturedVertex]>::from_iter(
            queue.device().clone(),
            BufferUsage::vertex_buffer(),
            false,
            vertices.into_iter(),
        )
        .unwrap();
        let index_buffer = CpuAccessibleBuffer::<[u32]>::from_iter(
            queue.device().clone(),
            BufferUsage::index_buffer(),
            false,
            indices.into_iter(),
        )
        .unwrap();

        let vs = vertex_shader::load(device.clone()).unwrap();
        let fs = fragment_shader::load(device.clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .build(device)
            .unwrap();

        Runner {
            queue,
            subpass,
            pipeline,
            vertices: vertex_buffer,
            indices: index_buffer,
        }
    }

    fn create_image_sampler_nearest(
        &self,
        image: Arc<dyn ImageViewAbstract>,
    ) -> Arc<PersistentDescriptorSet> {
        let layout = self.pipeline.layout().set_layouts().get(0).unwrap();
        let sampler = Sampler::new(
            self.queue.device().clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                address_mode: [SamplerAddressMode::Repeat; 3],
                mipmap_mode: SamplerMipmapMode::Nearest,
                ..Default::default()
            },
        )
        .unwrap();

        PersistentDescriptorSet::new(
            layout.clone(),
            [WriteDescriptorSet::image_view_sampler(
                0,
                image.clone(),
                sampler,
            )],
        )
        .unwrap()
    }

    pub fn draw(
        &self,
        size: [u32; 2],
        image: Arc<dyn ImageViewAbstract + 'static>,
    ) -> SecondaryAutoCommandBuffer {
        let mut builder = AutoCommandBufferBuilder::secondary(
            self.queue.device().clone(),
            self.queue.family(),
            CommandBufferUsage::MultipleSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();

        let desc_set = self.create_image_sampler_nearest(image);

        builder
            .set_viewport(
                0,
                [Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [size[0] as f32, size[1] as f32],
                    depth_range: 0.0..1.0,
                }],
            )
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                desc_set,
            )
            .bind_vertex_buffers(0, self.vertices.clone())
            .bind_index_buffer(self.indices.clone())
            .draw_indexed(self.indices.len() as u32, 1, 0, 0, 0)
            .unwrap();

        builder.build().unwrap()
    }
}
