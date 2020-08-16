use winit::dpi::PhysicalSize;
use winit::event::VirtualKeyCode;

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
}

impl FirstPersonController {
    pub fn new(mut camera: Camera, position: glm::Vec3) -> Self {
        camera.set_view(glm::translation(&position));
        Self {
            camera,
            position,
            direction: glm::vec3(0.0, 0.0, 1.0),
        }
    }

    pub fn handle_movement(&mut self, input_state: &InputState, dt: f32) {
        let movement_speed = 10.0;
        let rotation_speed = 0.5;

        let mut direction = glm::vec3(0.0, 0.0, 0.0);

        let mouse_delta = input_state.mouse_position().delta();

        self.direction = glm::rotate_y_vec3(&self.direction, -mouse_delta.x as f32 * rotation_speed * dt);
        let right = glm::cross(&self.direction, &glm::vec3(0.0, 1.0, 0.0)).normalize();
        self.direction =
            glm::rotate_vec3(&self.direction, mouse_delta.y as f32 * rotation_speed * dt, &right).normalize();

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

        let view = glm::look_at(
            &self.position,
            &(&self.position + &self.direction),
            &glm::vec3(0.0, 1.0, 0.0),
        );

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
