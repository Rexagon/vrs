use super::prelude::*;
use super::Device;

pub struct Buffer {
    size: vk::DeviceSize,
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}

impl Buffer {
    pub fn new(
        device: &Device,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        required_properties: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        // create buffer
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.handle().create_buffer(&buffer_create_info, None)? };
        log::debug!("created buffer {:?}", buffer);

        // find memory type
        let memory_requirements = device.get_buffer_memory_requirements(buffer);

        let memory_type = find_memory_type(
            device.memory_properties(),
            required_properties,
            memory_requirements.memory_type_bits,
        )?;

        // allocate memory
        let allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type);

        let memory = unsafe { device.handle().allocate_memory(&allocate_info, None)? };
        log::debug!("allocated buffer memory {:?}", memory);

        // bind buffer memory
        unsafe { device.handle().bind_buffer_memory(buffer, memory, 0)? };

        // done
        Ok(Self { size, buffer, memory })
    }

    pub unsafe fn destroy(&self, device: &Device) {
        let device = device.handle();

        device.destroy_buffer(self.buffer, None);
        log::debug!("dropped buffer {:?}", self.buffer);

        device.free_memory(self.memory, None);
        log::debug!("freed buffer memory {:?}", self.memory);
    }

    pub unsafe fn map_memory(&self, device: &Device) -> Result<*mut u8> {
        let data_ptr = device
            .handle()
            .map_memory(self.memory, 0, self.size, vk::MemoryMapFlags::empty())?;
        Ok(data_ptr as *mut u8)
    }

    pub unsafe fn unmap_memory(&self, device: &Device) {
        device.handle().unmap_memory(self.memory)
    }

    #[inline]
    pub fn size(&self) -> vk::DeviceSize {
        self.size
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
