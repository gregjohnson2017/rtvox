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
                        pressed_event!(MoveZ, Forward, Backward, camera.move_state.z)
                    }
                    VirtualKeyCode::A => pressed_event!(MoveX, Left, Right, camera.move_state.x),
                    VirtualKeyCode::S => {
                        pressed_event!(MoveZ, Backward, Forward, camera.move_state.z)
                    }
                    VirtualKeyCode::D => pressed_event!(MoveX, Right, Left, camera.move_state.x),
                    VirtualKeyCode::LShift => pressed_event!(MoveY, Down, Up, camera.move_state.y),
                    VirtualKeyCode::Space => pressed_event!(MoveY, Up, Down, camera.move_state.y),
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
                        released_event!(MoveZ, Forward, Backward, camera.move_state.z)
                    }
                    VirtualKeyCode::A => {
                        released_event!(MoveX, Left, Right, camera.move_state.x)
                    }
                    VirtualKeyCode::S => {
                        released_event!(MoveZ, Backward, Forward, camera.move_state.z)
                    }
                    VirtualKeyCode::D => {
                        released_event!(MoveX, Right, Left, camera.move_state.x)
                    }
                    VirtualKeyCode::LShift => {
                        released_event!(MoveY, Down, Up, camera.move_state.y)
                    }
                    VirtualKeyCode::Space => {
                        released_event!(MoveY, Up, Down, camera.move_state.y)
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

use paste::paste;

#[macro_export]
macro_rules! pressed_event {
    ( $ty:ty, $v:ident, $av:ident, $store:expr ) => {
        paste! {
            pressed_event!(
                $ty,
                $v,
                $av,
                [< $v Override >],
                [< $av Override >],
                $store
            )
        }
    };

    ( $ty:ty, $v:ident, $av:ident, $vo:ident, $avo:ident, $store:expr ) => {
        match $store {
            <$ty>::$v | <$ty>::$vo => (),
            <$ty>::$av => $store = <$ty>::$vo,
            <$ty>::$avo | <$ty>::None => $store = <$ty>::$v,
        }
    };
}

#[macro_export]
macro_rules! released_event {
    ( $ty:ty, $v:ident, $av:ident, $store:expr ) => {
        paste! {
            released_event!(
                $ty,
                $v,
                $av,
                [< $v Override >],
                [< $av Override >],
                $store
            )
        }
    };

    ( $ty:ty, $v:ident, $av:ident, $vo:ident, $avo:ident, $store:expr ) => {
        match $store {
            <$ty>::$v | <$ty>::None => $store = <$ty>::None,
            <$ty>::$vo | <$ty>::$avo => $store = <$ty>::$av,
            <$ty>::$av => (),
        }
    };
}
