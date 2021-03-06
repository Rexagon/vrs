use once_cell::sync::OnceCell;
use winit::dpi::{PhysicalPosition, PhysicalSize, Position};
use winit::event::{MouseButton, VirtualKeyCode};
use winit::window::Window;

use crate::input::InputState;

pub struct Camera {
    view: glm::Mat4,
    projection: glm::Mat4,
}

impl Camera {
    pub fn new(size: PhysicalSize<u32>) -> Self {
        let mut camera = Self {
            view: glm::identity(),
            projection: glm::identity(),
        };
        camera.update_projection(size);
        camera
    }

    #[inline]
    pub fn set_view(&mut self, view: glm::Mat4) {
        self.view = view;
    }

    #[inline]
    pub fn update_projection(&mut self, size: PhysicalSize<u32>) {
        let (width, height) = (size.width, size.height);

        self.projection = glm::perspective(width as f32 / height as f32, f32::to_radians(70.0), 0.01, 100.0);
        self.projection.m22 *= -1.0;
    }

    #[inline]
    pub fn view(&self) -> &glm::Mat4 {
        &self.view
    }

    #[inline]
    pub fn projection(&self) -> &glm::Mat4 {
        &self.projection
    }
}

pub struct FirstPersonController {
    pub camera: Camera,
    pub position: glm::Vec3,
    pub direction: glm::Vec3,
    pub relative_mouse_position: Option<PhysicalPosition<f64>>,
}

impl FirstPersonController {
    pub fn new(mut camera: Camera, position: glm::Vec3) -> Self {
        camera.set_view(glm::translation(&position));
        Self {
            camera,
            position,
            direction: glm::vec3(0.0, 0.0, 1.0),
            relative_mouse_position: None,
        }
    }

    pub fn handle_movement(&mut self, window: &Window, input_state: &InputState, dt: f32) {
        let movement_speed = 10.0;
        let rotation_speed = 0.5;

        let mut direction = glm::vec3(0.0, 0.0, 0.0);

        if self.relative_mouse_position.is_none() && input_state.mouse().is_pressed(MouseButton::Right) {
            self.relative_mouse_position = Some(input_state.mouse_position().current());
            window.set_cursor_visible(false);
        } else if self.relative_mouse_position.is_some() && input_state.mouse().is_released(MouseButton::Right) {
            self.relative_mouse_position = None;
            window.set_cursor_visible(true);
        }

        let right = if let Some(initial_mouse_position) = self.relative_mouse_position {
            let new_mouse_position = input_state.mouse_position().current();
            let mouse_delta = PhysicalPosition::new(
                new_mouse_position.x - initial_mouse_position.x,
                new_mouse_position.y - initial_mouse_position.y,
            );

            let _ = window.set_cursor_position(Position::Physical(PhysicalPosition::new(
                initial_mouse_position.x as i32,
                initial_mouse_position.y as i32,
            )));

            self.direction = glm::rotate_y_vec3(&self.direction, -mouse_delta.x as f32 * rotation_speed * dt);
            let right = glm::cross(&self.direction, direction_up()).normalize();
            self.direction =
                glm::rotate_vec3(&self.direction, mouse_delta.y as f32 * rotation_speed * dt, &right).normalize();
            right
        } else {
            glm::cross(&self.direction, direction_up()).normalize()
        };

        if input_state.keyboard().is_pressed(VirtualKeyCode::D) {
            direction += &right;
        } else if input_state.keyboard().is_pressed(VirtualKeyCode::A) {
            direction -= &right;
        }

        if input_state.keyboard().is_pressed(VirtualKeyCode::W) {
            direction += &self.direction;
        } else if input_state.keyboard().is_pressed(VirtualKeyCode::S) {
            direction -= &self.direction;
        }

        if direction != glm::vec3(0.0, 0.0, 0.0) {
            self.position += direction.normalize() * movement_speed * dt;
        }

        let view = glm::look_at(&self.position, &(self.position + self.direction), direction_up());

        self.camera.set_view(view);
    }

    #[inline]
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    #[inline]
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

fn direction_up() -> &'static glm::Vec3 {
    DIRECTION_UP.get_or_init(|| glm::vec3(0.0, 1.0, 0.0))
}

static DIRECTION_UP: OnceCell<glm::Vec3> = OnceCell::new();
