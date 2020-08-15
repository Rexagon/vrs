use anyhow::{Error, Result};
use ash::version::DeviceV1_0;
use ash::vk;

use crate::command_buffer::CommandPool;
use crate::logical_device::LogicalDevice;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

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

pub struct Mesh {
    index_count: u32,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl Mesh {
    pub fn new(
        logical_device: &LogicalDevice,
        command_pool: &CommandPool,
        vertices: &[Vertex],
        indices: &[u16],
    ) -> Result<Self> {
        let device = logical_device.handle();

        let vertex_buffer_size = std::mem::size_of_val(vertices) as vk::DeviceSize;
        let index_buffer_size = std::mem::size_of_val(indices) as vk::DeviceSize;
        let staging_buffer_size = vertex_buffer_size + index_buffer_size;

        // create staging buffer
        let staging_buffer = Buffer::new(
            logical_device,
            staging_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // write data to staging buffer
        unsafe {
            let data_ptr = device.map_memory(
                staging_buffer.memory(),
                0,
                staging_buffer_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut u8;

            let vertices_data = bytemuck::cast_slice(vertices);
            data_ptr
                .offset(0)
                .copy_from_nonoverlapping(vertices_data.as_ptr(), vertices_data.len());

            let indices_data = bytemuck::cast_slice(indices);
            data_ptr
                .offset(vertices_data.len() as isize)
                .copy_from_nonoverlapping(indices_data.as_ptr(), indices_data.len());

            assert_eq!(staging_buffer_size as usize, vertices_data.len() + indices_data.len());

            device.unmap_memory(staging_buffer.memory);
        }

        // create vertex buffer
        let vertex_buffer = Buffer::new(
            logical_device,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // create index buffer
        let index_buffer = Buffer::new(
            logical_device,
            index_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // copy data from staging to vertex buffer
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool.handle())
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };
        let command_buffer = command_buffers[0];

        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(command_buffer, &begin_info)?;

            let copy_regions = [vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: vertex_buffer_size,
            }];
            device.cmd_copy_buffer(
                command_buffer,
                staging_buffer.handle(),
                vertex_buffer.handle(),
                &copy_regions,
            );

            let copy_regions = [vk::BufferCopy {
                src_offset: vertex_buffer_size,
                dst_offset: 0,
                size: index_buffer_size,
            }];
            device.cmd_copy_buffer(
                command_buffer,
                staging_buffer.handle(),
                index_buffer.handle(),
                &copy_regions,
            );

            device.end_command_buffer(command_buffer)?;
        }

        let submit_info = [vk::SubmitInfo::builder().command_buffers(&command_buffers).build()];

        unsafe {
            device.queue_submit(logical_device.queues().graphics_queue, &submit_info, vk::Fence::null())?;
        }

        logical_device.wait_idle()?;

        unsafe {
            device.free_command_buffers(command_pool.handle(), &command_buffers);
        }

        // destroy staging buffer
        unsafe { staging_buffer.destroy(logical_device) };

        // done
        let index_count = indices.len() as u32;

        Ok(Self {
            index_count,
            vertex_buffer,
            index_buffer,
        })
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        self.vertex_buffer.destroy(logical_device);
        self.index_buffer.destroy(logical_device);
    }

    #[inline]
    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    #[inline]
    pub fn vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
    }

    #[inline]
    pub fn index_buffer(&self) -> &Buffer {
        &self.index_buffer
    }
}

pub struct Buffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}

impl Buffer {
    pub fn new(
        logical_device: &LogicalDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        required_properties: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        let device = logical_device.handle();

        // create buffer
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.create_buffer(&buffer_create_info, None)? };
        log::debug!("created buffer {:?}", buffer);

        // find memory type
        let memory_requirements = logical_device.get_buffer_memory_requirements(buffer);

        let memory_type = find_memory_type(
            logical_device.memory_properties(),
            required_properties,
            memory_requirements.memory_type_bits,
        )?;

        // allocate memory
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type);

        let memory = unsafe { device.allocate_memory(&allocate_info, None)? };
        log::debug!("allocated buffer memory {:?}", memory);

        // bind buffer memory
        unsafe { device.bind_buffer_memory(buffer, memory, 0)? };

        // done
        Ok(Self { buffer, memory })
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        let device = logical_device.handle();

        device.destroy_buffer(self.buffer, None);
        log::debug!("dropped buffer {:?}", self.buffer);

        device.free_memory(self.memory, None);
        log::debug!("freed buffer memory {:?}", self.memory);
    }

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self.buffer
    }

    #[inline]
    pub fn memory(&self) -> vk::DeviceMemory {
        self.memory
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

pub const QUAD_VERTICES: [Vertex; 4] = [
    Vertex {
        position: [0.0, 0.0, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        color: [0.5, 0.5, 0.0],
    },
];

pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];
