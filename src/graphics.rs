use std::{f32::consts::PI, sync::Arc};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, BlitImageInfo, ClearColorImageInfo, CommandBufferUsage,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{
        view::ImageView, ImageAccess, ImageDimensions, ImageLayout, ImageUsage, StorageImage,
        SwapchainImage,
    },
    memory::pool::StdMemoryPool,
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    swapchain::{
        acquire_next_image, AcquireError, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};

use winit::window::Window;

pub const COMPUTE_GROUP_SIZE: u32 = 8;
pub struct Graphics {
    surface: Arc<Surface<Window>>,
    pub recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
    storage_image: Arc<StorageImage<Arc<StdMemoryPool>>>,
    queue: Arc<Queue>,
    compute_pipeline: Arc<ComputePipeline>,
    camera_info: Arc<CpuAccessibleBuffer<cs::ty::CameraInfo>>,
}
impl Graphics {
    pub fn new(surface: Arc<Surface<Window>>) -> Self {
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        let (physical_device, queue_family) = PhysicalDevice::enumerate(surface.instance())
            .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| {
                        q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false)
                    })
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            })
            .expect("No suitable physical device found");

        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        // TODO [Rust Question] Why can't we add explicit type annotations here?
        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],

                ..DeviceCreateInfo::default()
            },
        )
        .unwrap();

        let queue = queues.next().unwrap();

        let image_format = Some(
            physical_device
                .surface_formats(&surface, SurfaceInfo::default())
                .unwrap()[0]
                .0,
        );

        let (swapchain, swapchain_images) = {
            let surface_capabilities = physical_device
                .surface_capabilities(&surface, SurfaceInfo::default())
                .unwrap();
            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,
                    image_format,
                    image_extent: surface.window().inner_size().into(),
                    image_usage: ImageUsage {
                        transfer_dst: true,
                        ..ImageUsage::color_attachment()
                    },
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .iter()
                        .next()
                        .unwrap(),
                    ..SwapchainCreateInfo::default()
                },
            )
            .unwrap()
        };

        let size = swapchain_images[0].dimensions().width_height();

        let storage_image = StorageImage::new(
            device.clone(),
            ImageDimensions::Dim2d {
                width: size[0],
                height: size[1],
                array_layers: 1,
            },
            Format::R8G8B8A8_UNORM,
            [queue.family()],
        )
        .unwrap();

        let camera_info = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage {
                uniform_buffer: true,
                ..BufferUsage::none()
            },
            false,
            cs::ty::CameraInfo {
                target: [0.0, 0.0, 5.0],
                fov: PI / 2.0,
                eye: [0.0, 0.0, 0.0],
            },
        )
        .unwrap();
        let cs = cs::load(device.clone()).unwrap();

        let compute_pipeline = ComputePipeline::new(
            device.clone(),
            cs.entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .unwrap();

        Self {
            surface,
            recreate_swapchain: false,
            previous_frame_end: Some(sync::now(device.clone()).boxed()),
            swapchain,
            swapchain_images,
            storage_image,
            queue,
            compute_pipeline,
            camera_info,
        }
    }

    pub fn redraw(&mut self) {
        let dimensions = self.surface.window().inner_size();
        if dimensions.width == 0 || dimensions.height == 0 {
            return;
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();
        let mut size = self.swapchain_images[0].dimensions().width_height();

        if self.recreate_swapchain {
            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };
            self.swapchain_images = new_images;
            self.swapchain = new_swapchain;
            self.recreate_swapchain = false;
            size = self.swapchain_images[0].dimensions().width_height();
            self.storage_image = StorageImage::new(
                self.queue.device().clone(),
                ImageDimensions::Dim2d {
                    width: size[0],
                    height: size[1],
                    array_layers: 1,
                },
                Format::R8G8B8A8_UNORM,
                [self.queue.family()],
            )
            .unwrap();
        }

        // This function can block if no image is available. The parameter is an optional timeout
        // after which the function call will return an error.
        let (next_image_idx, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        let future = self.previous_frame_end.take().unwrap().join(acquire_future);
        // let compute_future = compute_runner.compute(future.boxed());

        let mut builder = AutoCommandBufferBuilder::primary(
            self.queue.device().clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        let pipeline_layout = self.compute_pipeline.layout();
        let desc_layout = pipeline_layout.set_layouts().get(0).unwrap();
        let compute_desc_set = PersistentDescriptorSet::new(
            desc_layout.clone(),
            [
                WriteDescriptorSet::image_view(
                    0,
                    ImageView::new_default(self.storage_image.clone()).unwrap(),
                ),
                WriteDescriptorSet::buffer(1, self.camera_info.clone()),
            ],
        )
        .unwrap();

        builder
            .clear_color_image(ClearColorImageInfo::image(self.storage_image.clone()))
            .unwrap()
            .bind_pipeline_compute(self.compute_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.compute_pipeline.layout().clone(),
                0,
                compute_desc_set,
            )
            .dispatch([
                size[0] / COMPUTE_GROUP_SIZE,
                size[1] / COMPUTE_GROUP_SIZE,
                1,
            ])
            .unwrap()
            .blit_image(BlitImageInfo {
                src_image_layout: ImageLayout::General,
                dst_image_layout: ImageLayout::General,
                ..BlitImageInfo::images(
                    self.storage_image.clone(),
                    self.swapchain_images[next_image_idx].clone(),
                )
            })
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let render_future = future
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), next_image_idx)
            .then_signal_fence_and_flush();

        match render_future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.queue.device().clone()).boxed());
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.queue.device().clone()).boxed());
            }
        }
    }
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "src/graphics.comp",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}
