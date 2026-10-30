//! GPU kernels through wgpu (Metal, Vulkan, DX12): twins of CPU stages, which stay the
//! reference. A missing or failing GPU is an error the caller reports; nothing falls back
//! silently.

pub mod stack;

use crate::{Result, error};

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub name: String,
    max_buffer: u64,
    /// Errors outside any scope (recorded rather than panicking in wgpu's default handler).
    stray: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

/// Longest wait for one dispatch's results.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

impl Gpu {
    /// Names of the hardware adapters present.
    pub fn names() -> Vec<String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
            .iter()
            .map(|a| a.get_info())
            .filter(|i| i.device_type != wgpu::DeviceType::Cpu)
            .map(|i| format!("{} ({:?})", i.name, i.backend))
            .collect()
    }

    /// A device on the adapter whose name contains `name` (any case), or on the most capable
    /// one.
    pub fn new(name: Option<&str>) -> Result<Gpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = match name {
            Some(wanted) => {
                let lower = wanted.to_lowercase();
                pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
                    .into_iter()
                    .find(|a| a.get_info().name.to_lowercase().contains(&lower))
                    .ok_or_else(|| {
                        error(format!(
                            "no GPU named like \"{wanted}\"; present: {}",
                            Gpu::names().join(", ")
                        ))
                    })?
            }
            None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }))
            .map_err(|e| error(format!("no GPU adapter: {e}")))?,
        };
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu {
            return Err(error(format!(
                "only a software GPU adapter ({}) is available",
                info.name
            )));
        }
        let limits = adapter.limits();
        if limits.max_storage_buffers_per_shader_stage < 8 {
            return Err(error(format!(
                "{} offers too few storage buffers per stage",
                info.name
            )));
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("pictor"),
            required_limits: limits.clone(),
            ..Default::default()
        }))
        .map_err(|e| error(format!("{}: device creation failed ({e})", info.name)))?;
        let stray = std::sync::Arc::new(std::sync::Mutex::new(None));
        let record = stray.clone();
        device.on_uncaptured_error(std::sync::Arc::new(move |e| {
            if let Ok(mut slot) = record.lock() {
                slot.get_or_insert_with(|| e.to_string());
            }
        }));
        Ok(Gpu {
            device,
            queue,
            stray,
            name: format!("{} ({:?})", info.name, info.backend),
            max_buffer: limits
                .max_storage_buffer_binding_size
                .min(limits.max_buffer_size),
        })
    }

    /// Runs `f` inside out-of-memory, validation and internal error scopes; any error caught
    /// (or recorded outside scopes since) fails the call.
    pub fn scoped<T>(&self, what: &str, f: impl FnOnce() -> T) -> Result<T> {
        let scopes = [
            self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
            self.device.push_error_scope(wgpu::ErrorFilter::Validation),
            self.device.push_error_scope(wgpu::ErrorFilter::Internal),
        ];
        let value = f();
        let [oom, validation, internal] = scopes;
        let caught = [internal, validation, oom].map(|s| pollster::block_on(s.pop()));
        if let Some(e) = caught.into_iter().flatten().next() {
            return Err(error(format!("GPU {what}: {e:?}")));
        }
        if let Some(e) = self.stray.lock().ok().and_then(|s| s.clone()) {
            return Err(error(format!("GPU error: {e}")));
        }
        Ok(value)
    }

    pub fn storage(&self, label: &str, bytes: &[u8]) -> Result<wgpu::Buffer> {
        if bytes.len() as u64 > self.max_buffer {
            return Err(error(format!(
                "{label}: {} MiB exceeds the GPU's {} MiB buffer limit",
                bytes.len() >> 20,
                self.max_buffer >> 20
            )));
        }
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (bytes.len() as u64).max(16).next_multiple_of(4),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, bytes);
        Ok(buffer)
    }

    /// A storage buffer the kernel writes (not initialised from the host).
    pub fn output(&self, label: &str, size: u64) -> Result<wgpu::Buffer> {
        if size > self.max_buffer {
            return Err(error(format!(
                "{label}: {} MiB exceeds the GPU's {} MiB buffer limit",
                size >> 20,
                self.max_buffer >> 20
            )));
        }
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size.max(16),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }

    pub fn uniform(&self, label: &str, bytes: &[u8]) -> wgpu::Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (bytes.len() as u64).next_multiple_of(16),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, bytes);
        buffer
    }

    /// Submits `encoder` with a copy of the first `len` f32 values of `source` for reading
    /// back; the GPU works on while the caller does something else.
    pub fn submit_read(
        &self,
        mut encoder: wgpu::CommandEncoder,
        source: &wgpu::Buffer,
        len: usize,
    ) -> Pending {
        let size = (len * 4) as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: size.max(4),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(source, 0, &staging, 0, size);
        let index = self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..size)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        Pending {
            staging,
            size,
            index,
            done: rx,
        }
    }
}

/// A submitted readback.
pub struct Pending {
    staging: wgpu::Buffer,
    size: u64,
    index: wgpu::SubmissionIndex,
    done: std::sync::mpsc::Receiver<std::result::Result<(), wgpu::BufferAsyncError>>,
}

impl Pending {
    pub fn wait(self, gpu: &Gpu) -> Result<Vec<f32>> {
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(self.index),
                timeout: Some(TIMEOUT),
            })
            .map_err(|e| error(format!("GPU wait failed: {e}")))?;
        self.done
            .recv_timeout(std::time::Duration::from_secs(10))
            .map_err(|e| error(format!("GPU readback did not complete: {e}")))?
            .map_err(|e| error(format!("GPU readback failed: {e}")))?;
        let slice = self.staging.slice(..self.size);
        let data = slice
            .get_mapped_range()
            .map_err(|e| error(format!("GPU readback mapping failed: {e:?}")))?;
        let out = data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        drop(data);
        self.staging.unmap();
        Ok(out)
    }
}
