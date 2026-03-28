//! GPU acceleration over WebGPU.

mod monte_carlo;
mod splitting;
#[cfg(feature = "synthesis-gpu")]
mod synth_forward;
mod temporal;
mod transpiler;

pub(crate) use monte_carlo::{
    build_count_shader, build_temporal_shader, pack_noise_params, GpuMcContext,
};
pub use splitting::GpuSplittingEstimate;
#[cfg(feature = "synthesis-gpu")]
pub(crate) use synth_forward::{build_soft_forward_shader, SynthForwardContext};

use std::sync::OnceLock;

use pollster::FutureExt;

pub(crate) fn request_device_adapter() -> Option<wgpu::Adapter> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .block_on()
        .ok()?;
    (adapter.get_info().device_type != wgpu::DeviceType::Cpu).then_some(adapter)
}

/// Whether a usable GPU device is available.
#[must_use]
pub fn is_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        let Some(adapter) = request_device_adapter() else {
            return false;
        };
        adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .block_on()
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_check_never_panics() {
        let _ = is_available();
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn reports_a_present_device() {
        assert!(
            is_available(),
            "expected a usable GPU adapter on this node, but none was found"
        );
    }
}