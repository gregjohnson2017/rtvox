use std::sync::Arc;

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageAccess, ImageUsage, StorageImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::{ComputePipeline, Pipeline},
    swapchain::{
        acquire_next_image, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

struct ComputeRunner {
    storage_image: Arc<ImageView<StorageImage>>,
}

impl ComputeRunner {
    fn new(queue: Arc<Queue>, size: [u32; 2]) -> ComputeRunner {
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

        ComputeRunner { storage_image }
    }

    fn storage_image(&self) -> Arc<ImageView<StorageImage>> {
        self.storage_image.clone()
    }
}

fn main() {
    /*
    match vulkano::instance::layers_list() {
        Ok(iter) => {
            iter.for_each(|p| {
                println!("{}", p.name());
            });
        }
        Err(err) => {
            println!("LayersListError: {:?}", err);
            return;
        }
    }
    */
    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        enumerate_portability: true,
        enabled_layers: vec![String::from("VK_LAYER_LUNARG_monitor")],
        ..Default::default()
    })
    .unwrap();
    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false))
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

            ..Default::default()
        },
    )
    .unwrap();

    let queue = queues.next().unwrap();

    let (mut swapchain, images) = {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,
                image_format,
                image_extent: surface.window().inner_size().into(),
                image_usage: ImageUsage {
                    storage: true,
                    ..ImageUsage::color_attachment()
                },
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
    };

    let compute_runner = ComputeRunner::new(queue.clone(), images[0].dimensions().width_height());

    mod vs {
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

    mod fs {
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

    mod cs {
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

    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();
    let cs = cs::load(device.clone()).unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        cs.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    )
    .unwrap();

    let img_dims = images[0].dimensions().width_height();

    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => recreate_swapchain = true,
        Event::RedrawEventsCleared => {
            let dimensions = surface.window().inner_size();
            if dimensions.width == 0 || dimensions.height == 0 {
                return;
            }

            previous_frame_end.as_mut().unwrap().cleanup_finished();

            if recreate_swapchain {
                let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                    image_extent: dimensions.into(),
                    ..swapchain.create_info()
                }) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

                swapchain = new_swapchain;
                recreate_swapchain = false;
            }

            // This function can block if no image is available. The parameter is an optional timeout
            // after which the function call will return an error.
            let (image_num, suboptimal, acquire_future) =
                match acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            // Building a command buffer is an expensive operation (usually a few hundred
            // microseconds), but it is known to be a hot path in the driver and is expected to be
            // optimized.
            // TODO Could this be made outside the loop? Does it matter?
            let mut builder = AutoCommandBufferBuilder::primary(
                device.clone(),
                queue.family(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            let pipeline_layout = compute_pipeline.layout();
            let desc_layout = pipeline_layout.set_layouts().get(0).unwrap();
            let set = PersistentDescriptorSet::new(
                desc_layout.clone(),
                [WriteDescriptorSet::image_view(
                    0,
                    compute_runner.storage_image(),
                )],
            );
            builder
                .bind_pipeline_compute(compute_pipeline.clone())
                .dispatch([img_dims[0], img_dims[1], 1])
                .unwrap();

            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(future.boxed());
                }
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
            }
        }
        _ => (),
    });
}
