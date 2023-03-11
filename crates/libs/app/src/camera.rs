use std::time::Duration;

use crate::types::*;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, WindowEvent};

const MOVE_SPEED: f32 = 3.0;
const ANGLE_PER_POINT: f32 = 0.001745;

const FORWARD_SCANCODE: u32 = 17;
const BACKWARD_SCANCODE: u32 = 31;
const RIGHT_SCANCODE: u32 = 32;
const LEFT_SCANCODE: u32 = 30;
const UP_SCANCODE: u32 = 57;
const DOWN_SCANCODE: u32 = 29;

const UP: Vec3 = Vec3::new(0., 1., 0.);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub position: Point,
    pub direction: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(
        position: Point,
        direction: Vec3,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            fov,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    pub fn update(self, controls: &Controls, delta_time: Duration) -> Self {
        let delta_time = delta_time.as_secs_f32();
        let side = UniVec3::new_normalize(self.direction.cross(&UP));

        // Update direction
        let new_direction = if controls.look_around {
            let side_rot = Quat::from_axis_angle(&side, -controls.cursor_delta[1] * ANGLE_PER_POINT);
            let y_rot = Quat::from_axis_angle(&Vec3::y_axis(), -controls.cursor_delta[0] * ANGLE_PER_POINT);
            let rot = side_rot * y_rot;

            (rot * self.direction).normalize()
        } else {
            self.direction
        };
        // Update position
        let mut direction = Vec3::zeros();
        let side = side.into_inner();
        if controls.go_forward {
            direction += new_direction;
        }
        if controls.go_backward {
            direction -= new_direction;
        }
        if controls.strafe_right {
            direction += side;
        }
        if controls.strafe_left {
            direction -= side;
        }
        if controls.go_up {
            direction += UP;
        }
        if controls.go_down {
            direction -= UP;
        }

        let direction = if direction.norm_squared() == 0.0 {
            direction
        } else {
            direction.normalize()
        };

        Self {
            position: self.position + direction * MOVE_SPEED * delta_time,
            direction: new_direction,
            ..self
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            &self.position,
            &(self.position + self.direction),
            &Vec3::y(),
        )
    }

    pub fn projection_matrix(&self) -> Mat4 {
        OPENGL_TO_VULKAN_RT * Mat4::new_perspective(self.aspect_ratio, self.fov.to_radians(), self.z_near, self.z_far)

    }
}
const OPENGL_TO_VULKAN_RT: Mat4 = Mat4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[derive(Debug, Clone, Copy)]
pub struct Controls {
    pub go_forward: bool,
    pub go_backward: bool,
    pub strafe_right: bool,
    pub strafe_left: bool,
    pub go_up: bool,
    pub go_down: bool,
    pub look_around: bool,
    pub cursor_delta: [f32; 2],
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            go_forward: false,
            go_backward: false,
            strafe_right: false,
            strafe_left: false,
            go_up: false,
            go_down: false,
            look_around: false,
            cursor_delta: [0.0; 2],
        }
    }
}

impl Controls {
    pub fn reset(self) -> Self {
        Self {
            cursor_delta: [0.0; 2],
            ..self
        }
    }

    pub fn handle_event(self, event: &Event<()>) -> Self {
        let mut new_state = self;

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                scancode, state, ..
                            },
                        ..
                    } => {
                        if *scancode == FORWARD_SCANCODE {
                            new_state.go_forward = *state == ElementState::Pressed;
                        }
                        if *scancode == BACKWARD_SCANCODE {
                            new_state.go_backward = *state == ElementState::Pressed;
                        }
                        if *scancode == RIGHT_SCANCODE {
                            new_state.strafe_right = *state == ElementState::Pressed;
                        }
                        if *scancode == LEFT_SCANCODE {
                            new_state.strafe_left = *state == ElementState::Pressed;
                        }
                        if *scancode == UP_SCANCODE {
                            new_state.go_up = *state == ElementState::Pressed;
                        }
                        if *scancode == DOWN_SCANCODE {
                            new_state.go_down = *state == ElementState::Pressed;
                        }
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if *button == MouseButton::Right {
                            new_state.look_around = *state == ElementState::Pressed;
                        }
                    }
                    _ => {}
                };
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta: (x, y) },
                ..
            } => {
                let x = *x as f32;
                let y = *y as f32;
                new_state.cursor_delta = [self.cursor_delta[0] + x, self.cursor_delta[1] + y];
            }
            _ => (),
        }

        new_state
    }
}
