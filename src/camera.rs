use quaternion::Quaternion;
use vecmath::Vector3;

use crate::graphics::cs::ty::CameraInfo;

pub const FORWARD: Vector3<f32> = [0.0, 0.0, -1.0];
pub const BACKWARD: Vector3<f32> = [0.0, 0.0, 1.0];
pub const LEFT: Vector3<f32> = [-1.0, 0.0, 0.0];
pub const RIGHT: Vector3<f32> = [1.0, 0.0, 0.0];
pub const DOWN: Vector3<f32> = [0.0, -1.0, 0.0];
pub const UP: Vector3<f32> = [0.0, 1.0, 0.0];

const MOVEMENT_RATE: f32 = 0.1;

pub struct Camera {
    pos: Vector3<f32>,
    quat: Quaternion<f32>,
    fov: f32,
}

// right and down are angles in radians
pub struct LookEvent {
    pub right: f32,
    pub down: f32,
}

impl Camera {
    pub fn new(pos: Vector3<f32>, fov: f32) -> Self {
        Camera {
            pos: pos,
            quat: (1.0, [0.0, 0.0, 0.0]),
            fov,
        }
    }

    pub fn apply_look_event(&mut self, look_evt: LookEvent) {
        let quat_x = quaternion::axis_angle(DOWN, look_evt.right);
        self.quat = quaternion::mul(quat_x, self.quat);
        let ear_axis = quaternion::rotate_vector(self.quat, LEFT);
        let quat_y = quaternion::axis_angle(ear_axis, look_evt.down);
        self.quat = quaternion::mul(quat_y, self.quat);
    }

    // translation in a direction relative to current camera direction
    pub fn move_relative(&mut self, relative_dir: Vector3<f32>) {
        let dir = quaternion::rotate_vector(self.quat, relative_dir);
        self.move_absolute(dir)
    }

    // translation in an absolute direction
    pub fn move_absolute(&mut self, absolute_dir: Vector3<f32>) {
        let delta = vecmath::vec3_scale(absolute_dir, MOVEMENT_RATE);
        self.pos = vecmath::vec3_add(self.pos, delta);
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
}
