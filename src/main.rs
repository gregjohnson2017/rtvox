use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, ImageUsage, ImageViewAbstract, StorageImage},
    impl_vertex,
    instance::{Instance, InstanceCreateInfo},
    pipeline::{
        graphics::{
            input_assembly::{InputAssemblyState, PrimitiveTopology},
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        ComputePipeline, GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, Subpass},
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
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

    let (mut swapchain, mut images) = {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,
                image_format,
                image_extent: surface.window().inner_size().into(),
                image_usage: ImageUsage::color_attachment(),
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        let images = images
            .into_iter()
            .map(|image| ImageView::new_default(image.clone()).unwrap())
            .collect::<Vec<_>>();
        (swapchain, images)
    };

    let render_pass = vulkano::single_pass_renderpass!(queue.device().clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            }
        },
        pass: {
                color: [color],
                depth_stencil: {}
        }
    )
    .unwrap();
    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    let size = images[0].dimensions().width_height();

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

    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();

    let graphics_pipeline = GraphicsPipeline::start()
        .vertex_input_state(BuffersDefinition::new().vertex::<TexturedVertex>())
        .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleList))
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .render_pass(subpass.clone())
        .build(device.clone())
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
                images = new_images
                    .into_iter()
                    .map(|image| ImageView::new_default(image.clone()).unwrap())
                    .collect::<Vec<_>>();
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
            let compute_set = PersistentDescriptorSet::new(
                desc_layout.clone(),
                [WriteDescriptorSet::image_view(0, storage_image.clone())],
            )
            .unwrap();
            let layout = graphics_pipeline.layout().set_layouts().get(0).unwrap();
            let sampler = Sampler::new(
                queue.device().clone(),
                SamplerCreateInfo {
                    mag_filter: Filter::Nearest,
                    min_filter: Filter::Nearest,
                    address_mode: [SamplerAddressMode::Repeat; 3],
                    mipmap_mode: SamplerMipmapMode::Nearest,
                    ..Default::default()
                },
            )
            .unwrap();

            let graphics_set = PersistentDescriptorSet::new(
                layout.clone(),
                [WriteDescriptorSet::image_view_sampler(
                    0,
                    storage_image.clone(),
                    sampler,
                )],
            )
            .unwrap();
            let fb = Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![images[image_num].clone()],
                    ..Default::default()
                },
            )
            .unwrap();
            builder
                .bind_pipeline_compute(compute_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Compute,
                    compute_pipeline.layout().clone(),
                    0,
                    compute_set,
                )
                .dispatch([size[0], size[1], 1])
                .unwrap()
                .set_viewport(
                    0,
                    [Viewport {
                        origin: [0.0, 0.0],
                        dimensions: [size[0] as f32, size[1] as f32],
                        depth_range: 0.0..1.0,
                    }],
                )
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0., 0., 0., 1.].into())],
                        ..RenderPassBeginInfo::framebuffer(fb)
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .bind_pipeline_graphics(graphics_pipeline.clone())
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    graphics_pipeline.layout().clone(),
                    0,
                    graphics_set,
                )
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .bind_index_buffer(index_buffer.clone())
                .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                .unwrap()
                .end_render_pass()
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
