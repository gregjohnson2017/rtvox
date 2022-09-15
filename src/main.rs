use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, BlitImageInfo, ClearColorImageInfo, CommandBufferUsage,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageAccess, ImageDimensions, ImageLayout, ImageUsage, StorageImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    swapchain::{
        acquire_next_image, AcquireError, SurfaceInfo, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

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

    let (mut swapchain, mut images) = {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, SurfaceInfo::default())
            .unwrap();

        let (swapchain, images) = Swapchain::new(
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
        .unwrap();
        (swapchain, images)
    };

    let mut size = images[0].dimensions().width_height();

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

    mod cs {
        vulkano_shaders::shader! {
            ty: "compute",
            src: "
                    #version 450
    
                    layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;
    
                    void main() {
                        ivec2 dims = imageSize(img);
                        float r = 1.0 * float(gl_GlobalInvocationID.x) / float(dims.x);
                        float g = 1.0 * float(gl_GlobalInvocationID.y) / float(dims.y);
                        imageStore(img, ivec2(gl_GlobalInvocationID.xy), vec4(r, g, 0, 0));
                    }
                "
        }
    }

    let cs = cs::load(device.clone()).unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        cs.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    )
    .unwrap();

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
                images = new_images;
                swapchain = new_swapchain;
                recreate_swapchain = false;
                size = images[0].dimensions().width_height();
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

            let future = previous_frame_end.take().unwrap().join(acquire_future);
            // let compute_future = compute_runner.compute(future.boxed());

            let mut builder = AutoCommandBufferBuilder::primary(
                device.clone(),
                queue.family(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            let pipeline_layout = compute_pipeline.layout();
            let desc_layout = pipeline_layout.set_layouts().get(0).unwrap();
            let compute_desc_set = PersistentDescriptorSet::new(
                desc_layout.clone(),
                [WriteDescriptorSet::image_view(
                    0,
                    ImageView::new_default(storage_image.clone()).unwrap(),
                )],
            )
            .unwrap();

            builder
                .clear_color_image(ClearColorImageInfo::image(storage_image.clone()))
                .unwrap()
                .bind_pipeline_compute(compute_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Compute,
                    compute_pipeline.layout().clone(),
                    0,
                    compute_desc_set,
                )
                .dispatch([size[0], size[1], 1])
                .unwrap()
                .blit_image(BlitImageInfo {
                    src_image_layout: ImageLayout::General,
                    dst_image_layout: ImageLayout::General,
                    ..BlitImageInfo::images(storage_image.clone(), images[image_num].clone())
                })
                .unwrap();

            let command_buffer = builder.build().unwrap();

            let render_future = future
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                .then_signal_fence_and_flush();

            match render_future {
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
