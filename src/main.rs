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
                    VirtualKeyCode::W => {
                        use MoveZ::*;
                        match camera.move_state.z {
                            Forward | ForwardOverride => (),
                            Backward => camera.move_state.z = ForwardOverride,
                            BackwardOverride | None => camera.move_state.z = Forward,
                        }
                    }
                    VirtualKeyCode::A => {
                        use MoveX::*;
                        match camera.move_state.x {
                            Left | LeftOverride => (),
                            Right => camera.move_state.x = LeftOverride,
                            RightOverride | None => camera.move_state.x = Left,
                        }
                    }
                    VirtualKeyCode::S => {
                        use MoveZ::*;
                        match camera.move_state.z {
                            Backward | BackwardOverride => (),
                            Forward => camera.move_state.z = BackwardOverride,
                            ForwardOverride | None => camera.move_state.z = Backward,
                        }
                    }
                    VirtualKeyCode::D => {
                        use MoveX::*;
                        match camera.move_state.x {
                            Right | RightOverride => (),
                            Left => camera.move_state.x = RightOverride,
                            LeftOverride | None => camera.move_state.x = Right,
                        }
                    }
                    VirtualKeyCode::LShift => {
                        use MoveY::*;
                        match camera.move_state.y {
                            Down | DownOverride => (),
                            Up => camera.move_state.y = DownOverride,
                            UpOverride | None => camera.move_state.y = Down,
                        }
                    }
                    VirtualKeyCode::Space => {
                        use MoveY::*;
                        match camera.move_state.y {
                            Up | UpOverride => (),
                            Down => camera.move_state.y = UpOverride,
                            DownOverride | None => camera.move_state.y = Up,
                        }
                    }
                    _ => (),
                }
                match started_moving {
                    None if camera.is_moving() => started_moving = Some(Instant::now()),
                    _ => (),
                }
            }
            ElementState::Released => {
                match key {
                    VirtualKeyCode::W => {
                        use MoveZ::*;
                        match camera.move_state.z {
                            Forward | None => camera.move_state.z = None,
                            ForwardOverride | BackwardOverride => camera.move_state.z = Backward,
                            Backward => (),
                        }
                    }
                    VirtualKeyCode::A => {
                        use MoveX::*;
                        match camera.move_state.x {
                            Left | None => camera.move_state.x = None,
                            LeftOverride | RightOverride => camera.move_state.x = Right,
                            Right => (),
                        }
                    }
                    VirtualKeyCode::S => {
                        use MoveZ::*;
                        match camera.move_state.z {
                            Backward | None => camera.move_state.z = None,
                            BackwardOverride | ForwardOverride => camera.move_state.z = Forward,
                            Forward => (),
                        }
                    }
                    VirtualKeyCode::D => {
                        use MoveX::*;
                        match camera.move_state.x {
                            Right | None => camera.move_state.x = None,
                            RightOverride | LeftOverride => camera.move_state.x = Left,
                            Left => (),
                        }
                    }
                    VirtualKeyCode::LShift => {
                        use MoveY::*;
                        match camera.move_state.y {
                            Down | None => camera.move_state.y = None,
                            DownOverride | UpOverride => camera.move_state.y = Up,
                            Up => (),
                        }
                    }
                    VirtualKeyCode::Space => {
                        use MoveY::*;
                        match camera.move_state.y {
                            Up | None => camera.move_state.y = None,
                            UpOverride | DownOverride => camera.move_state.y = Down,
                            Down => (),
                        }
                    }
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
