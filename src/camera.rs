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
    dir: Option<MoveDirection>,
}

// right and down are angles in radians
pub struct LookEvent {
    pub right: f32,
    pub down: f32,
}

#[derive(Copy, Clone)]
pub enum MoveDirection {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

impl Camera {
    pub fn new(pos: Vector3<f32>, fov: f32) -> Self {
        Camera {
            pos: pos,
            quat: (1.0, [0.0, 0.0, 0.0]),
            fov,
            dir: None,
        }
    }

    pub fn apply_look_event(&mut self, look_evt: LookEvent) {
        let quat_x = quaternion::axis_angle(DOWN, look_evt.right);
        self.quat = quaternion::mul(quat_x, self.quat);
        let ear_axis = quaternion::rotate_vector(self.quat, LEFT);
        let quat_y = quaternion::axis_angle(ear_axis, look_evt.down);
        self.quat = quaternion::mul(quat_y, self.quat);
    }

    pub fn start_moving(&mut self, direction: MoveDirection) {
        self.dir = Some(direction)
    }

    pub fn update_position(&mut self, duration: Duration) {
        match self.dir {
            None => (),
            Some(dir) => {
                let (abs, dir) = match dir {
                    MoveDirection::Forward => (false, FORWARD),
                    MoveDirection::Backward => (false, BACKWARD),
                    MoveDirection::Left => (false, LEFT),
                    MoveDirection::Right => (false, RIGHT),
                    MoveDirection::Up => (true, UP),
                    MoveDirection::Down => (true, DOWN),
                };
                let movement = vecmath::vec3_scale(dir, duration.as_secs_f32());
                if abs {
                    self.move_absolute(movement)
                } else {
                    self.move_relative(movement);
                }
            }
        }
    }

    pub fn stop_moving(&mut self, duration: Duration) {
        self.update_position(duration);
        self.dir = None
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

        camera.stop_moving(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_eq!(info.eye, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_moving_from_position() {
        let mut camera = Camera::new([0.0, 1.0, 0.0], PI / 2.0);

        camera.start_moving(MoveDirection::Forward);
        camera.stop_moving(Duration::from_secs(1));

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
        camera.start_moving(MoveDirection::Forward);
        camera.stop_moving(Duration::from_secs(1));

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
        camera.start_moving(MoveDirection::Forward);
        camera.stop_moving(Duration::from_secs(1));

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
        camera.start_moving(MoveDirection::Up);
        camera.stop_moving(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_about_eq(info.eye, [0.0, 3.0, 0.0]);
    }

    #[test]
    fn test_update_position_short() {
        let mut camera = Camera::new([0.0, 0.0, 0.0], PI / 2.0);

        camera.start_moving(MoveDirection::Down);
        camera.update_position(Duration::from_secs(1));

        let info = camera.get_camera_info();
        assert_about_eq(info.eye, [0.0, -3.0, 0.0]);
    }

    #[test]
    fn test_moving_directions() {
        let tests: Vec<([f32; 3], MoveDirection, Duration, [f32; 3])> = vec![
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Forward,
                Duration::from_secs(1),
                [0.0, 0.0, -3.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Forward,
                Duration::from_secs(5),
                [0.0, 0.0, -15.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Backward,
                Duration::from_secs(1),
                [0.0, 0.0, 3.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Left,
                Duration::from_secs(2),
                [-6.0, 0.0, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Right,
                Duration::from_secs(4),
                [12.0, 0.0, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Up,
                Duration::from_secs_f32(0.5),
                [0.0, 1.5, 0.0],
            ),
            (
                [0.0, 0.0, 0.0],
                MoveDirection::Down,
                Duration::from_secs(2),
                [0.0, -6.0, 0.0],
            ),
        ];
        tests.iter().for_each(move |(pos, dir, dur, expect)| {
            let mut camera = Camera::new(*pos, PI / 2.0);

            camera.start_moving(*dir);
            camera.stop_moving(*dur);

            let info = camera.get_camera_info();
            assert_eq!(info.eye, *expect);
        });
    }
}
