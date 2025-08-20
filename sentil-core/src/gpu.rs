//! GPU acceleration over WebGPU.
//!
//! Large Monte Carlo and rare-event runs can be offloaded to a GPU through this
//! module. Everything here is gated behind the `gpu` feature and degrades
//! cleanly: when no compatible device is present, [`is_available`] returns
//! `false` and callers stay on the CPU path. Kernel execution needs real GPU
//! hardware, so it is exercised in a GPU session rather than on a headless host.

use pollster::FutureExt;

/// A GPU device and its command queue, acquired once and reused across runs.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl GpuContext {
    /// Acquires a GPU device, or `None` when no compatible adapter is present.
    ///
    /// This never fails loudly: a missing or unusable device yields `None` so the
    /// caller can fall back to the CPU rather than crash.
    #[must_use]
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .block_on()
            .ok()?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .block_on()
            .ok()?;
        Some(Self { device, queue })
    }

    /// The underlying device, for building buffers and pipelines.
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The command queue, for submitting work.
    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

/// Whether a GPU device is available, so the GPU statistical path can run.
#[must_use]
pub fn is_available() -> bool {
    GpuContext::new().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_check_never_panics() {
        // With no GPU this is false; with one, true. Either way it must not panic,
        // so the CPU fallback path stays reachable on any machine.
        let _ = is_available();
    }
}