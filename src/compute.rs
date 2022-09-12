use std::sync::Arc;

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{Device, Queue},
    format::Format,
    image::{view::ImageView, ImageUsage, StorageImage},
    pipeline::{ComputePipeline, Pipeline},
    sync::GpuFuture,
};

mod shader {
    vulkano_shaders::shader! {
        ty: "compute",
        src: "
                #version 450

                layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

                void main() {
                    imageStore(img, ivec2(gl_GlobalInvocationID.xy), vec4(255, 255, 255, 0));
                }
            "
    }
}

pub struct Runner {
    device: Arc<Device>,
    queue: Arc<Queue>,
    storage_image: Arc<ImageView<StorageImage>>,
    size: [u32; 2],
    pipeline: Arc<ComputePipeline>,
}

impl Runner {
    pub fn new(queue: Arc<Queue>, size: [u32; 2]) -> Runner {
        let device = queue.device().clone();

        let storage_image: Arc<ImageView<StorageImage>> = StorageImage::general_purpose_image_view(
            queue.clone(),
            size,
            Format::R8G8B8A8_UNORM,
            ImageUsage {
                sampled: true,
                transfer_dst: true,
                storage: true,
                color_attachment: true,
                ..ImageUsage::none()
            },
        )
        .unwrap();

        let cs = shader::load(device.clone()).unwrap();

        let pipeline = ComputePipeline::new(
            device.clone(),
            cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        Runner {
            device,
            queue,
            storage_image,
            size,
            pipeline,
        }
    }

    pub fn compute(&self, before_future: Box<dyn GpuFuture>) -> Box<dyn GpuFuture> {
        // Building a command buffer is an expensive operation (usually a few hundred
        // microseconds), but it is known to be a hot path in the driver and is expected to be
        // optimized.
        // TODO Could this be made outside the loop? Does it matter?
        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        let pipeline_layout = self.pipeline.layout();
        let desc_layout = pipeline_layout.set_layouts().get(0).unwrap();
        let set = PersistentDescriptorSet::new(
            desc_layout.clone(),
            [WriteDescriptorSet::image_view(
                0,
                self.storage_image.clone(),
            )],
        );
        builder
            .bind_pipeline_compute(self.pipeline.clone())
            .dispatch([self.size[0], self.size[1], 1])
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let after_future = before_future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .boxed();

        after_future
    }
}
