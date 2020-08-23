use std::sync::Arc;

use anyhow::Result;
use gltf::Gltf;

use crate::rendering::{CommandPool, Device, Mesh, Vertex};

pub struct Scene {
    meshes: Vec<Mesh>,
}

impl Scene {
    pub fn new<T>(device: Arc<Device>, command_pool: &CommandPool, path: T) -> Result<Self>
    where
        T: AsRef<std::path::Path>,
    {
        let loaded_data = Gltf::open(path)?;
        let blob = loaded_data.blob.as_ref().unwrap();

        let mut meshes = Vec::with_capacity(loaded_data.meshes().len());

        for (_, mesh) in loaded_data.meshes().enumerate() {
            let primitive = match mesh.primitives().next() {
                Some(primitive) => primitive,
                None => continue,
            };

            let reader = primitive.reader(|_| Some(blob));

            let vertices = match reader
                .read_positions()
                .and_then(|positions_iter| reader.read_normals().map(|normals_iter| (positions_iter, normals_iter)))
                .map(|(positions_iter, normals_iter)| {
                    positions_iter
                        .zip(normals_iter)
                        .map(|(position, normal)| Vertex {
                            position: [position[0], -position[2], position[1]],
                            normal: [normal[0], -normal[2], normal[1]],
                        })
                        .collect::<Vec<Vertex>>()
                }) {
                Some(vertices) => vertices,
                None => continue,
            };

            let indices: Vec<_> = match reader.read_indices().unwrap() {
                gltf::mesh::util::ReadIndices::U8(iter) => iter.map(|index| index as u16).collect(),
                gltf::mesh::util::ReadIndices::U16(iter) => iter.map(|index| index as u16).collect(),
                gltf::mesh::util::ReadIndices::U32(iter) => iter.map(|index| index as u16).collect(),
            };

            meshes.push(Mesh::new(device.clone(), command_pool, &vertices, &indices)?);
        }

        Ok(Self { meshes })
    }

    pub unsafe fn destroy(&self) {
        self.meshes.iter().for_each(|mesh| mesh.destroy());
    }

    #[inline]
    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }
}
