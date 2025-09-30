//! Synthesis: smooth robustness, models, the solver, controllers, and numerics.

use crate::conversions::{clear_error, ffi_panic_boundary, slice_from};
use crate::{SentilError, SentilSoftKind};
use libc::{c_void, size_t};
use sentil::synthesis::{soft_max, soft_min, solve_qp, solve_spd, symmetric_eigen, SmoothConfig};
use sentil::{Formula, Trace};

/// Smoothing settings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilSmoothConfig {
    pub temperature: f64,
    pub kind: SentilSoftKind,
}

impl SentilSmoothConfig {
    fn to_core(self) -> sentil::Result<SmoothConfig> {
        Ok(SmoothConfig::new(self.temperature)?.with_kind(self.kind.into()))
    }
}

#[no_mangle]
pub extern "C" fn sentil_smooth_config_default() -> SentilSmoothConfig {
    let d = SmoothConfig::default();
    SentilSmoothConfig { temperature: d.temperature(), kind: d.kind().into() }
}

#[no_mangle]
pub extern "C" fn sentil_soft_min(values: *const f64, n: size_t, temperature: f64) -> f64 {
    clear_error();
    ffi_panic_boundary(f64::NAN, || {
        let Ok(values) = slice_from(values, n) else {
            return f64::NAN;
        };
        soft_min(values, temperature)
    })
}

#[no_mangle]
pub extern "C" fn sentil_soft_max(values: *const f64, n: size_t, temperature: f64) -> f64 {
    clear_error();
    ffi_panic_boundary(f64::NAN, || {
        let Ok(values) = slice_from(values, n) else {
            return f64::NAN;
        };
        soft_max(values, temperature)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_smooth_robustness(
    formula: *mut c_void,
    trace: *mut c_void,
    config: *const SentilSmoothConfig,
    out: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        let config = match unsafe { *config }.to_core() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };
        match formula.smooth_robustness(trace, config) {
            Ok(value) => {
                unsafe { *out = value };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

fn matrix_from(data: *const f64, rows: usize, cols: usize) -> Result<Vec<Vec<f64>>, SentilError> {
    let flat = slice_from(data, rows.saturating_mul(cols))?;
    Ok((0..rows).map(|r| flat[r * cols..(r + 1) * cols].to_vec()).collect())
}

#[no_mangle]
pub extern "C" fn sentil_solve_qp(
    p: *const f64,
    n: size_t,
    q: *const f64,
    g: *const f64,
    m: size_t,
    h: *const f64,
    max_iters: size_t,
    out: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let p = match matrix_from(p, n, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let g = match matrix_from(g, m, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let (Ok(q), Ok(h)) = (slice_from(q, n), slice_from(h, m)) else {
            return SentilError::NullPointer;
        };
        match solve_qp(&p, q, &g, h, max_iters) {
            Ok(u) => {
                unsafe { std::ptr::copy_nonoverlapping(u.as_ptr(), out, u.len()) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_solve_spd(
    matrix: *const f64,
    n: size_t,
    rhs: *const f64,
    out: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let matrix = match matrix_from(matrix, n, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let Ok(rhs) = slice_from(rhs, n) else {
            return SentilError::NullPointer;
        };
        match solve_spd(&matrix, rhs) {
            Ok(x) => {
                unsafe { std::ptr::copy_nonoverlapping(x.as_ptr(), out, x.len()) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_symmetric_eigen(
    matrix: *const f64,
    n: size_t,
    out_values: *mut f64,
    out_vectors: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out_values, SentilError::NullPointer);
        check_ptr!(out_vectors, SentilError::NullPointer);
        let matrix = match matrix_from(matrix, n, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        match symmetric_eigen(&matrix) {
            Ok((values, vectors)) => {
                unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), out_values, values.len()) };
                for (j, row) in vectors.iter().enumerate() {
                    unsafe {
                        std::ptr::copy_nonoverlapping(row.as_ptr(), out_vectors.add(j * n), row.len())
                    };
                }
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}