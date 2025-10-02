use crate::conversions::{clear_error, ffi_panic_boundary};
use crate::stats::SentilRareEventConfig;
use crate::SentilError;
use libc::{c_void, size_t};
use sentil::stats::{RareEventConfig, SimModel};
use sentil::Formula;

#[no_mangle]
pub extern "C" fn sentil_gpu_is_available() -> bool {
    clear_error();
    ffi_panic_boundary(false, sentil::gpu::is_available)
}

/// A fixed-effort multilevel-splitting estimate from the GPU.
#[repr(C)]
pub struct SentilGpuSplittingEstimate {
    pub violation_probability: f64,
    pub particles: size_t,
    pub levels: u32,
}

#[no_mangle]
pub extern "C" fn sentil_formula_check_rare_event_gpu(
    formula: *mut c_void,
    model: *mut c_void,
    config: *const SentilRareEventConfig,
    out: *mut SentilGpuSplittingEstimate,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let model = borrow_handle!(model, SimModel, SentilError::NullPointer);
        let config: RareEventConfig = unsafe { *config }.into();
        match formula.check_rare_event_gpu(model, &config) {
            Ok(estimate) => {
                unsafe {
                    *out = SentilGpuSplittingEstimate {
                        violation_probability: estimate.violation_probability,
                        particles: estimate.particles,
                        levels: estimate.levels,
                    };
                }
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}