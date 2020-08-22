use super::prelude::*;
use super::{Buffer, CommandPool, Device};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
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
                offset: offset_of!(Self, normal) as u32,
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
    pub fn new(device: &Device, command_pool: &CommandPool, vertices: &[Vertex], indices: &[u16]) -> Result<Self> {
        let vertex_buffer_size = std::mem::size_of_val(vertices) as vk::DeviceSize;
        let index_buffer_size = std::mem::size_of_val(indices) as vk::DeviceSize;
        let staging_buffer_size = vertex_buffer_size + index_buffer_size;

        // create staging buffer
        let staging_buffer = Buffer::new(
            device,
            staging_buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // write data to staging buffer
        unsafe {
            let data_ptr = device.handle().map_memory(
                staging_buffer.memory().handle(),
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

            device.handle().unmap_memory(staging_buffer.memory().handle());
        }

        // create vertex buffer
        let vertex_buffer = Buffer::new(
            device,
            vertex_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // create index buffer
        let index_buffer = Buffer::new(
            device,
            index_buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // copy data from staging to vertex buffer
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool.handle())
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers = unsafe { device.handle().allocate_command_buffers(&allocate_info)? };
        let command_buffer = command_buffers[0];

        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.handle().begin_command_buffer(command_buffer, &begin_info)?;

            let copy_regions = [vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size: vertex_buffer_size,
            }];
            device.handle().cmd_copy_buffer(
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
            device.handle().cmd_copy_buffer(
                command_buffer,
                staging_buffer.handle(),
                index_buffer.handle(),
                &copy_regions,
            );

            device.handle().end_command_buffer(command_buffer)?;
        }

        let submit_info = [vk::SubmitInfo::builder().command_buffers(&command_buffers).build()];

        unsafe {
            device
                .handle()
                .queue_submit(device.queues().graphics_queue, &submit_info, vk::Fence::null())?;
        }

        device.wait_idle()?;

        unsafe {
            device
                .handle()
                .free_command_buffers(command_pool.handle(), &command_buffers);
        }

        // destroy staging buffer
        unsafe { staging_buffer.destroy(device) };

        // done
        let index_count = indices.len() as u32;

        Ok(Self {
            index_count,
            vertex_buffer,
            index_buffer,
        })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        self.vertex_buffer.destroy(device);
        self.index_buffer.destroy(device);
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

#[allow(unused)]
pub const QUAD_VERTICES: [Vertex; 4] = [
    Vertex {
        position: [0.0, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        normal: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        normal: [0.5, 0.5, 0.0],
    },
];

#[allow(unused)]
pub const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];
