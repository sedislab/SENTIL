//! Synthesis: smooth robustness, models, the solver, controllers, and numerics.

use crate::conversions::{clear_error, ffi_panic_boundary, slice_from};
use crate::SentilError;
use libc::size_t;
use sentil::synthesis::{solve_qp, solve_spd, symmetric_eigen};

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