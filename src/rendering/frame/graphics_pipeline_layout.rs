use crate::rendering::prelude::*;
use crate::rendering::{Buffer, Device};

pub struct GraphicsPipelineLayout {
    device: Arc<Device>,
    descriptor_pool: Arc<DescriptorPool>,
    pipeline_layout: vk::PipelineLayout,
    uniform_buffers: UniformBuffers,
}

impl GraphicsPipelineLayout {
    pub fn new(device: Arc<Device>, max_frames_in_flight: usize) -> Result<Self> {
        let descriptor_pool = Arc::new(DescriptorPool::new(device.clone(), max_frames_in_flight)?);
        let uniform_buffers = UniformBuffers::new(device.clone(), descriptor_pool.clone(), max_frames_in_flight)?;

        let descriptor_set_layouts = [uniform_buffers.layout()];
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);

        let pipeline_layout = unsafe {
            device
                .handle()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };
        log::debug!("created pipeline layout {:?}", pipeline_layout);

        Ok(Self {
            device,
            descriptor_pool,
            pipeline_layout,
            uniform_buffers,
        })
    }

    pub unsafe fn destroy(&self) {
        self.device.handle().destroy_pipeline_layout(self.pipeline_layout, None);
        log::debug!("dropped pipeline layout {:?}", self.pipeline_layout);

        self.uniform_buffers.destroy();
        self.descriptor_pool.destroy();
    }

    #[inline]
    pub fn handle(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }

    #[inline]
    pub fn uniform_buffers(&self) -> &UniformBuffers {
        &self.uniform_buffers
    }

    #[inline]
    pub fn uniform_buffers_mut(&mut self) -> &mut UniformBuffers {
        &mut self.uniform_buffers
    }
}

pub struct UniformBuffers {
    device: Arc<Device>,
    descriptor_pool: Arc<DescriptorPool>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    world_data_buffers: Vec<Buffer>,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl UniformBuffers {
    pub fn new(device: Arc<Device>, descriptor_pool: Arc<DescriptorPool>, max_frames_in_flight: usize) -> Result<Self> {
        // create descriptor set layout
        let ubo_layout_bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build()];

        let ubo_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&ubo_layout_bindings);

        let descriptor_set_layout = unsafe {
            device
                .handle()
                .create_descriptor_set_layout(&ubo_layout_create_info, None)?
        };
        log::debug!("created descriptor set layout {:?}", descriptor_set_layout);

        // create buffers
        let buffer_size = (std::mem::size_of::<glm::Mat4>() * 2) as vk::DeviceSize;

        let world_data_buffers =
            (0..max_frames_in_flight).try_fold(Vec::with_capacity(max_frames_in_flight), |mut buffers, _| {
                Buffer::new(
                    device.clone(),
                    buffer_size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
                .map(|buffer| {
                    buffers.push(buffer);
                    buffers
                })
            })?;

        // create descriptor sets
        let layouts = std::iter::repeat(descriptor_set_layout)
            .take(max_frames_in_flight)
            .collect::<Vec<_>>();

        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool.handle())
            .set_layouts(&layouts);
        let descriptor_sets = unsafe {
            device
                .handle()
                .allocate_descriptor_sets(&descriptor_set_allocate_info)?
        };

        // bind descriptor sets to buffers
        for (i, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let descriptor_buffer_info = [vk::DescriptorBufferInfo {
                buffer: world_data_buffers[i].handle(),
                offset: 0,
                range: world_data_buffers[i].size(),
            }];

            let descriptor_write_sets = [vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&descriptor_buffer_info)
                .build()];

            unsafe {
                device.handle().update_descriptor_sets(&descriptor_write_sets, &[]);
            }
        }

        // done
        Ok(Self {
            device,
            descriptor_pool,
            descriptor_set_layout,
            world_data_buffers,
            descriptor_sets,
        })
    }

    pub unsafe fn destroy(&self) {
        self.world_data_buffers.iter().for_each(|buffer| buffer.destroy());

        let device = self.device.handle();

        device.free_descriptor_sets(self.descriptor_pool.handle(), &self.descriptor_sets);

        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        log::debug!("dropped descriptor set layout {:?}", self.descriptor_set_layout);
    }

    pub fn update_world_data(&mut self, current_frame: usize, view: &glm::Mat4, projection: &glm::Mat4) -> Result<()> {
        let buffer = &self.world_data_buffers[current_frame];

        unsafe {
            let data_ptr = buffer.map_memory()?;

            let mut buffer_data = [0f32; 16 * 2];
            buffer_data[..16].copy_from_slice(view.as_slice());
            buffer_data[16..].copy_from_slice(projection.as_slice());
            let buffer_data_slice = bytemuck::cast_slice(&buffer_data);

            data_ptr.copy_from_nonoverlapping(buffer_data_slice.as_ptr(), buffer_data_slice.len());

            buffer.unmap_memory();
        }

        Ok(())
    }

    #[inline]
    pub fn descriptor_set(&self, current_frame: usize) -> vk::DescriptorSet {
        self.descriptor_sets[current_frame]
    }

    #[inline]
    pub fn layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

pub struct DescriptorPool {
    device: Arc<Device>,
    descriptor_pool: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(device: Arc<Device>, size: usize) -> Result<Self> {
        let pool_sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: size as u32,
        }];

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(size as u32)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {
            device
                .handle()
                .create_descriptor_pool(&descriptor_pool_create_info, None)?
        };
        log::debug!("created descriptor pool {:?}", descriptor_pool);

        Ok(Self {
            device,
            descriptor_pool,
        })
    }

    pub unsafe fn destroy(&self) {
        self.device.handle().destroy_descriptor_pool(self.descriptor_pool, None);
        log::debug!("dropped descriptor pool {:?}", self.descriptor_pool);
    }

    #[inline]
    pub fn handle(&self) -> vk::DescriptorPool {
        self.descriptor_pool
    }
}
