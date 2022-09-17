use std::{f32::consts::PI, time::Instant};

use camera::{Camera, LookEvent, MoveX, MoveY, MoveZ};
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
        } => match state {
            ElementState::Pressed => {
                match key {
                    VirtualKeyCode::W => match camera.move_state.z {
                        MoveZ::Forward | MoveZ::ForwardOverride => (),
                        MoveZ::Backward => camera.move_state.z = MoveZ::ForwardOverride,
                        MoveZ::BackwardOverride | MoveZ::None => {
                            camera.move_state.z = MoveZ::Forward
                        }
                    },
                    VirtualKeyCode::A => match camera.move_state.x {
                        MoveX::Left | MoveX::LeftOverride => (),
                        MoveX::Right => camera.move_state.x = MoveX::LeftOverride,
                        MoveX::RightOverride | MoveX::None => camera.move_state.x = MoveX::Left,
                    },
                    VirtualKeyCode::S => match camera.move_state.z {
                        MoveZ::Backward | MoveZ::BackwardOverride => (),
                        MoveZ::Forward => camera.move_state.z = MoveZ::BackwardOverride,
                        MoveZ::ForwardOverride | MoveZ::None => {
                            camera.move_state.z = MoveZ::Backward
                        }
                    },
                    VirtualKeyCode::D => match camera.move_state.x {
                        MoveX::Right | MoveX::RightOverride => (),
                        MoveX::Left => camera.move_state.x = MoveX::RightOverride,
                        MoveX::LeftOverride | MoveX::None => camera.move_state.x = MoveX::Right,
                    },
                    VirtualKeyCode::LShift => match camera.move_state.y {
                        MoveY::Down | MoveY::DownOverride => (),
                        MoveY::Up => camera.move_state.y = MoveY::DownOverride,
                        MoveY::UpOverride | MoveY::None => camera.move_state.y = MoveY::Down,
                    },
                    VirtualKeyCode::Space => match camera.move_state.y {
                        MoveY::Up | MoveY::UpOverride => (),
                        MoveY::Down => camera.move_state.y = MoveY::UpOverride,
                        MoveY::DownOverride | MoveY::None => camera.move_state.y = MoveY::Up,
                    },
                    _ => (),
                }
                match started_moving {
                    None if camera.is_moving() => started_moving = Some(Instant::now()),
                    _ => (),
                }
            }
            ElementState::Released => {
                match key {
                    VirtualKeyCode::W => match camera.move_state.z {
                        MoveZ::Forward | MoveZ::None => camera.move_state.z = MoveZ::None,
                        MoveZ::ForwardOverride | MoveZ::BackwardOverride => {
                            camera.move_state.z = MoveZ::Backward
                        }
                        MoveZ::Backward => (),
                    },
                    VirtualKeyCode::A => match camera.move_state.x {
                        MoveX::Left | MoveX::None => camera.move_state.x = MoveX::None,
                        MoveX::LeftOverride | MoveX::RightOverride => {
                            camera.move_state.x = MoveX::Right
                        }
                        MoveX::Right => (),
                    },
                    VirtualKeyCode::S => match camera.move_state.z {
                        MoveZ::Backward | MoveZ::None => camera.move_state.z = MoveZ::None,
                        MoveZ::BackwardOverride | MoveZ::ForwardOverride => {
                            camera.move_state.z = MoveZ::Forward
                        }
                        MoveZ::Forward => (),
                    },
                    VirtualKeyCode::D => match camera.move_state.x {
                        MoveX::Right | MoveX::None => camera.move_state.x = MoveX::None,
                        MoveX::RightOverride | MoveX::LeftOverride => {
                            camera.move_state.x = MoveX::Left
                        }
                        MoveX::Left => (),
                    },
                    VirtualKeyCode::LShift => match camera.move_state.y {
                        MoveY::Down | MoveY::None => camera.move_state.y = MoveY::None,
                        MoveY::DownOverride | MoveY::UpOverride => camera.move_state.y = MoveY::Up,
                        MoveY::Up => (),
                    },
                    VirtualKeyCode::Space => match camera.move_state.y {
                        MoveY::Up | MoveY::None => camera.move_state.y = MoveY::None,
                        MoveY::UpOverride | MoveY::DownOverride => {
                            camera.move_state.y = MoveY::Down
                        }
                        MoveY::Down => (),
                    },
                    _ => (),
                }
                match started_moving {
                    Some(_) if !camera.is_moving() => started_moving = None,
                    _ => (),
                }
            }
        },
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
