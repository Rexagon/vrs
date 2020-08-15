use anyhow::{Error, Result};
use ash::version::DeviceV1_0;
use ash::vk;

use crate::command_buffer::CommandPool;
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

pub struct Mesh {
    index_count: u32,
    vertex_buffer: Buffer,
}

impl Mesh {
    pub fn new(logical_device: &LogicalDevice, command_pool: &CommandPool, vertices: &[Vertex]) -> Result<Self> {
        let device = logical_device.handle();

        let index_count = vertices.len() as u32;
        let buffer_size = std::mem::size_of_val(vertices) as vk::DeviceSize;

        let staging_buffer = Buffer::new(
            logical_device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let data_ptr =
                device.map_memory(staging_buffer.memory(), 0, buffer_size, vk::MemoryMapFlags::empty())? as *mut Vertex;

            data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len());

            device.unmap_memory(staging_buffer.memory);
        }

        let vertex_buffer = Buffer::new(
            logical_device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            logical_device,
            command_pool,
            staging_buffer.handle(),
            vertex_buffer.handle(),
            buffer_size,
        )?;

        unsafe { staging_buffer.destroy(logical_device) };

        Ok(Self {
            index_count,
            vertex_buffer,
        })
    }

    pub unsafe fn destroy(&self, logical_device: &LogicalDevice) {
        self.vertex_buffer.destroy(logical_device);
    }

    #[inline]
    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    #[inline]
    pub fn vertex_buffer(&self) -> &Buffer {
        &self.vertex_buffer
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
        log::debug!("dropped vertex buffer {:?}", self.buffer);

        device.free_memory(self.memory, None);
        log::debug!("freed vertex buffer memory {:?}", self.memory);
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

pub fn copy_buffer(
    logical_device: &LogicalDevice,
    command_pool: &CommandPool,
    src_buffer: vk::Buffer,
    dst_buffer: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<()> {
    let device = logical_device.handle();

    let allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool.handle())
        .command_buffer_count(1)
        .level(vk::CommandBufferLevel::PRIMARY);

    let command_buffers = unsafe { device.allocate_command_buffers(&allocate_info)? };
    let command_buffer = command_buffers[0];

    let begin_info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe {
        device.begin_command_buffer(command_buffer, &begin_info)?;

        let copy_regions = [vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size,
        }];
        device.cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &copy_regions);

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

    Ok(())
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

pub const TRIANGLE: [Vertex; 3] = [
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
