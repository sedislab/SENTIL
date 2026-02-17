//! On-board planning, receding-horizon control, and input shielding.

use alloc::boxed::Box;
use alloc::vec::Vec;

use sentil::synthesis::{soft_max, soft_min, solve_qp, solve_spd, symmetric_eigen};
use sentil::{
    Backend, Bounds, Controller, Formula, LinearModel, SafetyFilter, SynthesisProblem, SystemModel,
    Synthesizer,
};

use crate::{read_slice, status_of, Status};

unsafe fn read_matrix(ptr: *const f64, rows: usize, cols: usize) -> Option<Vec<Vec<f64>>> {
    let total = rows.checked_mul(cols)?;
    let flat = read_slice(ptr, total)?;
    Some((0..rows).map(|r| flat[r * cols..(r + 1) * cols].to_vec()).collect())
}

unsafe fn write_out(out: *mut f64, values: &[f64]) {
    core::ptr::copy_nonoverlapping(values.as_ptr(), out, values.len());
}

/// Solves `matrix * x = rhs` for a symmetric positive-definite `matrix`, writing
/// `n` values to `out`. `matrix` is row-major `n`-by-`n`.
///
/// # Safety
///
/// `matrix` points to `n*n` doubles, `rhs` and `out` to `n` each.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_solve_spd(
    matrix: *const f64,
    n: usize,
    rhs: *const f64,
    out: *mut f64,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    let (Some(a), Some(b)) = (read_matrix(matrix, n, n), read_slice(rhs, n)) else {
        return Status::NullPointer;
    };
    match solve_spd(&a, b) {
        Ok(x) => {
            write_out(out, &x);
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Eigen-decomposes a symmetric row-major `n`-by-`n` `matrix`, writing the `n`
/// eigenvalues to `out_values` and the eigenvectors row-major to `out_vectors`.
///
/// # Safety
///
/// `matrix` and `out_vectors` point to `n*n` doubles, `out_values` to `n`.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_symmetric_eigen(
    matrix: *const f64,
    n: usize,
    out_values: *mut f64,
    out_vectors: *mut f64,
) -> Status {
    if out_values.is_null() || out_vectors.is_null() {
        return Status::NullPointer;
    }
    let Some(a) = read_matrix(matrix, n, n) else {
        return Status::NullPointer;
    };
    match symmetric_eigen(&a) {
        Ok((values, vectors)) => {
            write_out(out_values, &values);
            for (r, row) in vectors.iter().enumerate() {
                write_out(out_vectors.add(r * n), row);
            }
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Solves the quadratic program that minimises `0.5 x' P x + q' x` subject to
/// `G x <= h`, writing `n` values to `out`. `P` is `n`-by-`n`, `G` is `m`-by-`n`.
///
/// # Safety
///
/// The pointers cover their stated shapes; `out` holds `n` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_solve_qp(
    p: *const f64,
    n: usize,
    q: *const f64,
    g: *const f64,
    h: *const f64,
    m: usize,
    out: *mut f64,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    let (Some(p), Some(q), Some(g), Some(h)) =
        (read_matrix(p, n, n), read_slice(q, n), read_matrix(g, m, n), read_slice(h, m))
    else {
        return Status::NullPointer;
    };
    const QP_DUAL_ITERS: usize = 200;
    match solve_qp(&p, q, &g, h, QP_DUAL_ITERS) {
        Ok(x) => {
            write_out(out, &x);
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// A smooth lower bound on the minimum of `n` values at the given temperature.
///
/// # Safety
///
/// `values` points to `n` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_soft_min(values: *const f64, n: usize, temperature: f64) -> f64 {
    match read_slice(values, n) {
        Some(values) => soft_min(values, temperature),
        None => f64::NAN,
    }
}

/// A smooth upper bound on the maximum of `n` values at the given temperature.
///
/// # Safety
///
/// `values` points to `n` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_soft_max(values: *const f64, n: usize, temperature: f64) -> f64 {
    match read_slice(values, n) {
        Some(values) => soft_max(values, temperature),
        None => f64::NAN,
    }
}

/// Box bounds with a per-coordinate lower and upper limit, writing the handle to
/// `*out`.
///
/// # Safety
///
/// `lower` and `upper` point to `n` doubles, `out` to a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bounds_create(
    lower: *const f64,
    upper: *const f64,
    n: usize,
    out: *mut *mut Bounds,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    let (Some(lower), Some(upper)) = (read_slice(lower, n), read_slice(upper, n)) else {
        return Status::NullPointer;
    };
    match Bounds::new(lower.to_vec(), upper.to_vec()) {
        Ok(bounds) => {
            *out = Box::into_raw(Box::new(bounds));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Bounds that constrain nothing over `dimension` coordinates.
///
/// # Safety
///
/// `out` points to a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bounds_unbounded(dimension: usize, out: *mut *mut Bounds) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = Box::into_raw(Box::new(Bounds::unbounded(dimension)));
    Status::Ok
}

/// Frees a bounds handle. A null pointer is a no-op.
///
/// # Safety
///
/// `bounds` must be a live handle not already destroyed or consumed by a filter.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bounds_destroy(bounds: *mut Bounds) {
    if !bounds.is_null() {
        drop(Box::from_raw(bounds));
    }
}

/// Builds a linear time-invariant model `x_{t+1} = A x_t + B u_t`, writing the
/// handle to `*out`. `a` is row-major `n`-by-`n`, `b` is `n`-by-`b_cols`, `x0`
/// has length `n`.
///
/// # Safety
///
/// The pointers cover their stated shapes; `variables` is `n` null-terminated
/// strings; `out` is a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_linear_model_create(
    a: *const f64,
    n: usize,
    b: *const f64,
    b_cols: usize,
    x0: *const f64,
    variables: *const *const core::ffi::c_char,
    dt: f64,
    horizon: usize,
    out: *mut *mut LinearModel,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    if variables.is_null() {
        return Status::NullPointer;
    }
    let (Some(a), Some(b), Some(x0)) = (read_matrix(a, n, n), read_matrix(b, n, b_cols), read_slice(x0, n)) else {
        return Status::NullPointer;
    };
    let mut names: Vec<&str> = Vec::with_capacity(n);
    for i in 0..n {
        let ptr = *variables.add(i);
        if ptr.is_null() {
            return Status::NullPointer;
        }
        match core::ffi::CStr::from_ptr(ptr).to_str() {
            Ok(name) => names.push(name),
            Err(_) => return Status::InvalidConfig,
        }
    }
    match LinearModel::new(a, b, x0.to_vec(), names, dt, horizon) {
        Ok(model) => {
            *out = Box::into_raw(Box::new(model));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// The total number of input values the model takes across its horizon.
///
/// # Safety
///
/// `model` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_model_input_dimension(model: *const LinearModel) -> usize {
    if model.is_null() {
        0
    } else {
        (*model).input_dimension()
    }
}

/// Frees a model handle. A null pointer is a no-op.
///
/// # Safety
///
/// `model` is a live handle not already destroyed or consumed by a controller.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_model_destroy(model: *mut LinearModel) {
    if !model.is_null() {
        drop(Box::from_raw(model));
    }
}

fn backend_of(code: core::ffi::c_int) -> Backend {
    match code {
        1 => Backend::Gradient,
        2 => Backend::CmaEs,
        _ => Backend::Auto,
    }
}

/// Plans an input sequence for the model that best satisfies the spec, writing
/// the input to `out_input` (length the model's input dimension), the robustness
/// to `*out_robustness`, and whether it holds to `*out_holds`.
///
/// `backend` is 0 for automatic, 1 for gradient, 2 for CMA-ES.
///
/// # Safety
///
/// `out_input` holds at least the model's input dimension; the other out-pointers
/// are writable; the handles are live.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_synthesize(
    model: *const LinearModel,
    spec: *const Formula,
    bounds: *const Bounds,
    backend: core::ffi::c_int,
    max_iters: usize,
    out_input: *mut f64,
    out_robustness: *mut f64,
    out_holds: *mut bool,
) -> Status {
    if model.is_null() || spec.is_null() || out_input.is_null() {
        return Status::NullPointer;
    }
    let mut problem = SynthesisProblem::new(&*model, &*spec).with_backend(backend_of(backend));
    if !bounds.is_null() {
        let bounds = &*bounds;
        match Bounds::new(bounds.lower().to_vec(), bounds.upper().to_vec()) {
            Ok(rebuilt) => problem = problem.with_bounds(rebuilt),
            Err(e) => return status_of(&e),
        }
    }
    if max_iters != 0 {
        problem = problem.with_budget(max_iters);
    }
    match Synthesizer::solve(&problem) {
        Ok(result) => {
            write_out(out_input, &result.input);
            if !out_robustness.is_null() {
                *out_robustness = result.robustness;
            }
            if !out_holds.is_null() {
                *out_holds = result.holds;
            }
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// A controller plus the model and spec it owns.
pub struct EmbeddedController {
    controller: Controller<'static, LinearModel>,
    model: *mut LinearModel,
    spec: *mut Formula,
}

unsafe fn free_model_and_spec(model: *mut LinearModel, spec: *mut Formula) {
    if !model.is_null() {
        drop(Box::from_raw(model));
    }
    if !spec.is_null() {
        drop(Box::from_raw(spec));
    }
}

/// Builds an online receding-horizon controller over the model and spec, applying
/// `input_width` values per step and spending at most `max_iters` gradient steps
/// planning each step. The model and spec are consumed, even on failure.
///
/// # Safety
///
/// `model` and `spec` are live handles surrendered to the controller; `bounds` is
/// a live handle or null; `out` is a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_controller_create(
    model: *mut LinearModel,
    spec: *mut Formula,
    input_width: usize,
    max_iters: usize,
    bounds: *const Bounds,
    out: *mut *mut EmbeddedController,
) -> Status {
    if !out.is_null() {
        *out = core::ptr::null_mut();
    }
    if out.is_null() || model.is_null() || spec.is_null() {
        free_model_and_spec(model, spec);
        return Status::NullPointer;
    }
    let rebuilt = if bounds.is_null() {
        None
    } else {
        let bounds = &*bounds;
        match Bounds::new(bounds.lower().to_vec(), bounds.upper().to_vec()) {
            Ok(rebuilt) => Some(rebuilt),
            Err(e) => {
                free_model_and_spec(model, spec);
                return status_of(&e);
            }
        }
    };
    // Box::into_raw already leaked the handles, so the 'static borrow holds.
    let mut controller: Controller<'static, LinearModel> =
        Controller::with_iterations(&*model, &*spec, input_width, max_iters);
    if let Some(rebuilt) = rebuilt {
        controller = controller.with_bounds(rebuilt);
    }
    *out = Box::into_raw(Box::new(EmbeddedController { controller, model, spec }));
    Status::Ok
}

/// Plans from the current `state` (length the number of state variables) and
/// writes the first input, of length the controller's input width, to `out`.
///
/// # Safety
///
/// `controller` is a live handle, `state` points to its state width, and `out`
/// holds at least the input width.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_controller_control(
    controller: *mut EmbeddedController,
    state: *const f64,
    n: usize,
    out: *mut f64,
) -> Status {
    if controller.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    let Some(state) = read_slice(state, n) else {
        return Status::NullPointer;
    };
    match (*controller).controller.control(state) {
        Ok(input) => {
            write_out(out, &input);
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Frees a controller and the model and spec it owns. A null pointer is a no-op.
///
/// # Safety
///
/// `controller` is a live handle not already destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_controller_destroy(controller: *mut EmbeddedController) {
    if controller.is_null() {
        return;
    }
    let EmbeddedController { controller, model, spec } = *Box::from_raw(controller);
    drop(controller);
    drop(Box::from_raw(model));
    drop(Box::from_raw(spec));
}

/// A safety filter and the input width its bounds fix.
pub struct EmbeddedSafetyFilter {
    filter: SafetyFilter,
    width: usize,
}

/// Builds a safety filter that keeps inputs inside `bounds`, which it consumes,
/// writing the handle to `*out`.
///
/// # Safety
///
/// `bounds` is a live handle surrendered to the filter; `out` is a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_safety_filter_create(
    bounds: *mut Bounds,
    out: *mut *mut EmbeddedSafetyFilter,
) -> Status {
    if out.is_null() {
        if !bounds.is_null() {
            drop(Box::from_raw(bounds));
        }
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    if bounds.is_null() {
        return Status::NullPointer;
    }
    let bounds = *Box::from_raw(bounds);
    let width = bounds.dimension();
    *out = Box::into_raw(Box::new(EmbeddedSafetyFilter { filter: SafetyFilter::new(bounds), width }));
    Status::Ok
}

/// Writes the input closest to `nominal` (length `n`) that satisfies each barrier
/// `a_i . u >= b_i` and the bounds, to `out`. `barrier_a` is row-major `m`-by-`n`
/// and `barrier_b` has length `m`; pass `m` of 0 for bounds-only clamping.
///
/// # Safety
///
/// The pointers cover their stated shapes; `out` holds `n` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_safety_filter_filter(
    filter: *const EmbeddedSafetyFilter,
    nominal: *const f64,
    n: usize,
    barrier_a: *const f64,
    barrier_b: *const f64,
    m: usize,
    out: *mut f64,
) -> Status {
    if filter.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    if (*filter).width != n {
        return Status::InvalidConfig;
    }
    let (Some(nominal), Some(rows), Some(values)) =
        (read_slice(nominal, n), read_matrix(barrier_a, m, n), read_slice(barrier_b, m))
    else {
        return Status::NullPointer;
    };
    let barriers: Vec<(Vec<f64>, f64)> = rows.into_iter().zip(values.iter().copied()).collect();
    match (*filter).filter.filter(nominal, &barriers) {
        Ok(input) => {
            write_out(out, &input);
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Frees a safety filter. A null pointer is a no-op.
///
/// # Safety
///
/// `filter` must be a live handle that has not already been destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_safety_filter_destroy(filter: *mut EmbeddedSafetyFilter) {
    if !filter.is_null() {
        drop(Box::from_raw(filter));
    }
}