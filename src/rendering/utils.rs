#[derive(Default, Debug, Clone)]
pub struct ScreenVertex {
    pub position: [f32; 2],
}
vulkano::impl_vertex!(ScreenVertex, position);

impl ScreenVertex {
    pub fn quad() -> [Self; 4] {
        [
            ScreenVertex { position: [-1.0, -1.0] },
            ScreenVertex { position: [1.0, -1.0] },
            ScreenVertex { position: [1.0, 1.0] },
            ScreenVertex { position: [-1.0, 1.0] },
        ]
    }
}

pub mod screen_vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/screen.vert"
    }
}
