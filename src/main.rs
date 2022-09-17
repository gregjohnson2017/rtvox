use std::{f32::consts::PI, time::Instant};

use camera::{Camera, LookEvent, MoveDirection};
use graphics::Graphics;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::PhysicalSize,
    event::KeyboardInput,
    event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod graphics;

fn main() {
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
        .with_min_inner_size(PhysicalSize {
            width: graphics::COMPUTE_GROUP_SIZE,
            height: graphics::COMPUTE_GROUP_SIZE,
        })
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);
    let mut graphics = Graphics::new(surface, camera.get_camera_info());
    let mut mouse_1_held = false;
    let mut started_moving: Option<Instant> = None;
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,

        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => graphics.recreate_swapchain = true,

        Event::RedrawEventsCleared => {
            match started_moving {
                None => (),
                Some(dur) => {
                    camera.update_position(dur.elapsed());
                    started_moving = Some(Instant::now());
                }
            }
            graphics.update_camera(camera.get_camera_info());
            graphics.redraw();
        }

        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(key),
                            ..
                        },
                    ..
                },
            ..
        } => {
            let direction = match key {
                VirtualKeyCode::W => Some(MoveDirection::Forward),
                VirtualKeyCode::A => Some(MoveDirection::Left),
                VirtualKeyCode::S => Some(MoveDirection::Backward),
                VirtualKeyCode::D => Some(MoveDirection::Right),
                VirtualKeyCode::LShift => Some(MoveDirection::Down),
                VirtualKeyCode::Space => Some(MoveDirection::Up),
                _ => None,
            };
            match direction {
                None => (),
                Some(direction) => match state {
                    ElementState::Pressed => {
                        started_moving = Some(Instant::now());
                        camera.start_moving(direction);
                    }
                    ElementState::Released => {
                        started_moving.map(|i| camera.stop_moving(i.elapsed()));
                        started_moving.take();
                    }
                },
            }
        }
        Event::DeviceEvent {
            // dx and dy are in "unspecified units"
            event: DeviceEvent::MouseMotion { delta: (dx, dy) },
            ..
        } if mouse_1_held => {
            let look_evt = LookEvent {
                right: dx as f32 / 500.0,
                down: dy as f32 / 500.0,
            };
            camera.apply_look_event(look_evt);
        }
        Event::WindowEvent {
            event:
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                },
            ..
        } => match state {
            ElementState::Pressed => mouse_1_held = true,
            ElementState::Released => mouse_1_held = false,
        },
        _ => (),
    });
}
