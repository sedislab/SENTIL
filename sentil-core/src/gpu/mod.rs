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

use pollster::FutureExt;

/// Whether a usable GPU device is available, so the GPU statistical path can run.
#[must_use]
pub fn is_available() -> bool {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .block_on()
    else {
        return false;
    };
    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .block_on()
        .is_ok()
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