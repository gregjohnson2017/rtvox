use quaternion::Quaternion;
use std::time::Duration;
use vecmath::Vector3;

use crate::graphics::cs::ty::CameraInfo;

const FORWARD: Vector3<f32> = [0.0, 0.0, -1.0];
const BACKWARD: Vector3<f32> = [0.0, 0.0, 1.0];
const LEFT: Vector3<f32> = [-1.0, 0.0, 0.0];
const RIGHT: Vector3<f32> = [1.0, 0.0, 0.0];
const DOWN: Vector3<f32> = [0.0, -1.0, 0.0];
const UP: Vector3<f32> = [0.0, 1.0, 0.0];

const MOVEMENT_RATE: f32 = 3.0;

pub struct Camera {
    pos: Vector3<f32>,
    quat: Quaternion<f32>,
    fov: f32,
    pub move_state: MoveState,
}

// right and down are angles in radians
pub struct LookEvent {
    pub right: f32,
    pub down: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct MoveState {
    pub x: MoveX,
    pub y: MoveY,
    pub z: MoveZ,
}

impl Default for MoveState {
    fn default() -> MoveState {
        MoveState {
            x: MoveX::None,
            y: MoveY::None,
            z: MoveZ::None,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MoveX {
    Left,
    LeftOverride,
    Right,
    RightOverride,
    None,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MoveY {
    Up,
    UpOverride,
    Down,
    DownOverride,
    None,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum MoveZ {
    Forward,
    ForwardOverride,
    Backward,
    BackwardOverride,
    None,
}

impl Camera {
    pub fn new(pos: Vector3<f32>, fov: f32) -> Self {
        Camera {
            pos: pos,
            quat: (1.0, [0.0, 0.0, 0.0]),
            fov,
            move_state: MoveState::default(),
        }
    }

    pub fn apply_look_event(&mut self, look_evt: LookEvent) {
        let quat_x = quaternion::axis_angle(DOWN, look_evt.right);
        self.quat = quaternion::mul(quat_x, self.quat);
        let ear_axis = quaternion::rotate_vector(self.quat, LEFT);
        let quat_y = quaternion::axis_angle(ear_axis, look_evt.down);
        self.quat = quaternion::mul(quat_y, self.quat);
    }

    pub fn update_position(&mut self, dur: Duration) {
        {
            use MoveX::*;
            match self.move_state.x {
                Left | LeftOverride => {
                    self.move_relative(vecmath::vec3_scale(LEFT, dur.as_secs_f32()))
                }
                Right | RightOverride => {
                    self.move_relative(vecmath::vec3_scale(RIGHT, dur.as_secs_f32()))
                }
                None => (),
            }
        }
        {
            use MoveY::*;
            match self.move_state.y {
                Up | UpOverride => self.move_absolute(vecmath::vec3_scale(UP, dur.as_secs_f32())),
                Down | DownOverride => {
                    self.move_absolute(vecmath::vec3_scale(DOWN, dur.as_secs_f32()))
                }
                None => (),
            }
        }
        {
            use MoveZ::*;
            match self.move_state.z {
                Forward | ForwardOverride => {
                    self.move_relative(vecmath::vec3_scale(FORWARD, dur.as_secs_f32()))
                }
                Backward | BackwardOverride => {
                    self.move_relative(vecmath::vec3_scale(BACKWARD, dur.as_secs_f32()))
                }
                None => (),
            }
        }
    }

    pub fn get_camera_info(&self) -> CameraInfo {
        let dir = quaternion::rotate_vector(self.quat, FORWARD);
        let target = vecmath::vec3_add(self.pos, dir);
        CameraInfo {
            target,
            fov: self.fov,
            eye: self.pos,
        }
    }

    pub fn is_moving(&self) -> bool {
        self.move_state.x != MoveX::None
            || self.move_state.y != MoveY::None
            || self.move_state.z != MoveZ::None
    }

    // translation in a direction relative to current camera direction
    fn move_relative(&mut self, relative_dir: Vector3<f32>) {
        let dir = quaternion::rotate_vector(self.quat, relative_dir);
        self.move_absolute(dir)
    }

    // translation in an absolute direction
    fn move_absolute(&mut self, absolute_dir: Vector3<f32>) {
        let delta = vecmath::vec3_scale(absolute_dir, MOVEMENT_RATE);
        self.pos = vecmath::vec3_add(self.pos, delta);
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;
    use MoveX::*;
    use MoveY::*;
    use MoveZ::*;

    fn assert_about_eq(left: Vector3<f32>, right: Vector3<f32>) {
        const TOLERANCE: f32 = 0.001;
        for i in 0..3 {
            if !(left[i] - TOLERANCE < right[i]) || !(left[i] + TOLERANCE > right[i]) {
                panic!("index {}: {} !~ {}\n", i, left[i], right[i]);
            }
        }
    }

    #[test]
    fn test_stop_moving_doesnt_move() {
        let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);

        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_eq!(info.eye, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_moving_from_position() {
        let mut camera = Camera::new([0.0, 1.0, 0.0], PI / 2.0);

        camera.move_state = MoveState {
            z: Forward,
            ..Default::default()
        };
        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_eq!(info.eye, [0.0, 1.0, -3.0]);
    }

    #[test]
    fn test_moving_from_turned() {
        let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);

        camera.apply_look_event(LookEvent {
            right: PI,
            down: 0.0,
        });
        camera.move_state = MoveState {
            z: Forward,
            ..Default::default()
        };
        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_about_eq(info.eye, [0.0, 0.0, 3.0]);
    }

    #[test]
    fn test_moving_forward_looking_down() {
        let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);

        camera.apply_look_event(LookEvent {
            right: 0.0,
            down: PI / 2.0,
        });
        camera.move_state = MoveState {
            z: Forward,
            ..Default::default()
        };
        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_about_eq(info.eye, [0.0, -3.0, 0.0]);
    }

    #[test]
    fn test_moving_up_is_absolute() {
        let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);

        camera.apply_look_event(LookEvent {
            right: 0.0,
            down: PI / 2.0,
        });
        camera.move_state = MoveState {
            y: Up,
            ..Default::default()
        };
        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_about_eq(info.eye, [0.0, 3.0, 0.0]);
    }

    #[test]
    fn test_update_position_short() {
        let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);

        camera.move_state = MoveState {
            y: Down,
            ..Default::default()
        };
        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_about_eq(info.eye, [0.0, -3.0, 0.0]);
    }

    #[test]
    fn test_moving_directions() {
        let tests: Vec<([f32; 3], MoveState, Duration, [f32; 3])> = vec![
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    z: Forward,
                    ..Default::default()
                },
                Duration::from_secs(1),
                [0.0, 0.0, -3.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    z: Forward,
                    ..Default::default()
                },
                Duration::from_secs(5),
                [0.0, 0.0, -15.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    z: Backward,
                    ..Default::default()
                },
                Duration::from_secs(1),
                [0.0, 0.0, 3.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    x: Left,
                    ..Default::default()
                },
                Duration::from_secs(2),
                [-6.0, 0.0, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    x: Right,
                    ..Default::default()
                },
                Duration::from_secs(4),
                [12.0, 0.0, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    y: Up,
                    ..Default::default()
                },
                Duration::from_secs_f32(0.5),
                [0.0, 1.5, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveState {
                    y: Down,
                    ..Default::default()
                },
                Duration::from_secs(2),
                [0.0, -6.0, 0.0],
            ),
        ];
        tests.iter().for_each(move |(pos, dir, dur, expect)| {
            let mut camera = Camera::new(*pos, PI / 2.0);

            camera.move_state = *dir;
            camera.update_position(*dur);

            let info = camera.get_camera_info();
            assert_eq!(info.eye, *expect);
        });
    }
}
