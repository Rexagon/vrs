use anyhow::{Error, Result};
use ash::version::DeviceV1_0;
use ash::vk;

use crate::logical_device::LogicalDevice;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, color) as u32,
            },
        ]
    }
}

pub struct VertexBuffer {
    index_count: u32,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
}

impl VertexBuffer {
    pub fn new(logical_device: &LogicalDevice) -> Result<Self> {
        let device = logical_device.handle();

        let index_count = TRIANGLE.len() as u32;

        //
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(std::mem::size_of_val(&TRIANGLE) as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let vertex_buffer = unsafe { logical_device.handle().create_buffer(&buffer_create_info, None)? };
        log::debug!("created vertex buffer {:?}", vertex_buffer);

        //
        let memory_requirements = logical_device.get_buffer_memory_requirements(vertex_buffer);

        let memory_type = find_memory_type(
            logical_device.memory_properties(),
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            memory_requirements.memory_type_bits,
        )?;

        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type);

        let vertex_buffer_memory = unsafe { device.allocate_memory(&allocate_info, None)? };
        log::debug!("allocated vertex buffer memory {:?}", vertex_buffer_memory);

        //
        unsafe {
            device.bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)?;

            let mapped_memory = device.map_memory(
                vertex_buffer_memory,
                0,
                buffer_create_info.size,
                vk::MemoryMapFlags::empty(),
            )? as *mut Vertex;

            mapped_memory.copy_from_nonoverlapping(TRIANGLE.as_ptr(), TRIANGLE.len());

            device.unmap_memory(vertex_buffer_memory);
        }

        Ok(Self {
            index_count,
            vertex_buffer,
            vertex_buffer_memory,
        })
    }

    #[inline]
    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self.vertex_buffer
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        let device = logical_device.handle();

        device.destroy_buffer(self.vertex_buffer, None);
        log::debug!("dropped vertex buffer {:?}", self.vertex_buffer);

        device.free_memory(self.vertex_buffer_memory, None);
        log::debug!("freed vertex buffer memory {:?}", self.vertex_buffer_memory);
    }
}

pub fn find_memory_type(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    required_properties: vk::MemoryPropertyFlags,
    type_filter: u32,
) -> Result<u32> {
    for (i, memory_type) in memory_properties.memory_types.iter().enumerate() {
        if (type_filter & (1 << i)) > 0 && memory_type.property_flags.contains(required_properties) {
            return Ok(i as u32);
        }
    }

    Err(Error::msg("failed to find suitable memory type"))
}

const TRIANGLE: [Vertex; 3] = [
    Vertex {
        position: [0.0, -0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];
