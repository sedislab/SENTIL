use crate::conversions::{clear_error, collect_strings, ffi_panic_boundary, set_error, slice_from};
use crate::handles::{drop_handle, into_boxed_array, into_handle, take_handle};
use crate::{SentilBackend, SentilError, SentilSoftKind};
use libc::{c_char, c_void, size_t};
use sentil::stats::StochasticSystem;
use sentil::synthesis::{
    cma_es, cma_es_batched, maximize, soft_max, soft_min, solve_qp, solve_spd, symmetric_eigen,
    AffineForm, Bounds, ChanceConstraint, ChanceReport, CmaConfig, Controller, LinearModel,
    SafetyFilter, SmoothConfig, Synthesizer, SynthesisProblem, SystemModel,
};
use sentil::{Formula, Trace};
use std::time::Duration;

type ModelHandle = Box<dyn SystemModel>;

struct DynModel<'a>(&'a dyn SystemModel);

impl SystemModel for DynModel<'_> {
    fn input_dimension(&self) -> usize {
        self.0.input_dimension()
    }

    fn initial_state(&self) -> &[f64] {
        self.0.initial_state()
    }

    fn rollout_from(&self, initial: &[f64], input: &[f64]) -> sentil::Result<Trace> {
        self.0.rollout_from(initial, input)
    }

    fn affine_form(&self) -> Option<AffineForm> {
        self.0.affine_form()
    }
}

struct CustomModel {
    variables: Vec<String>,
    dt: f64,
    horizon: usize,
    initial: Vec<f64>,
    input_dim: usize,
    userdata: usize,
    rollout: unsafe extern "C" fn(*mut c_void, *const f64, size_t, *const f64, size_t, *mut f64),
}

impl SystemModel for CustomModel {
    fn input_dimension(&self) -> usize {
        self.input_dim
    }

    fn initial_state(&self) -> &[f64] {
        &self.initial
    }

    fn rollout_from(&self, initial: &[f64], input: &[f64]) -> sentil::Result<Trace> {
        let samples = self.horizon + 1;
        let n_vars = self.variables.len();
        let mut signals = vec![0.0_f64; n_vars * samples];
        unsafe {
            (self.rollout)(
                self.userdata as *mut c_void,
                initial.as_ptr(),
                initial.len(),
                input.as_ptr(),
                input.len(),
                signals.as_mut_ptr(),
            );
        }
        let times: Vec<f64> = (0..samples).map(|i| i as f64 * self.dt).collect();
        let mut trace = Trace::new(times)?;
        for (v, name) in self.variables.iter().enumerate() {
            trace.add_signal(name, signals[v * samples..(v + 1) * samples].to_vec())?;
        }
        Ok(trace)
    }
}

/// Callbacks defining a custom system model.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilModelVtable {
    pub userdata: *mut c_void,
    pub input_dimension: size_t,
    pub initial_state: *const f64,
    pub rollout:
        Option<unsafe extern "C" fn(*mut c_void, *const f64, size_t, *const f64, size_t, *mut f64)>,
}

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

#[no_mangle]
pub extern "C" fn sentil_bounds_create(
    lower: *const f64,
    upper: *const f64,
    n: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || {
        let (Ok(lower), Ok(upper)) = (slice_from(lower, n), slice_from(upper, n)) else {
            return std::ptr::null_mut();
        };
        match Bounds::new(lower.to_vec(), upper.to_vec()) {
            Ok(bounds) => into_handle(bounds),
            Err(e) => {
                let _: SentilError = e.into();
                std::ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_bounds_unbounded(dimension: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || into_handle(Bounds::unbounded(dimension)))
}

#[no_mangle]
pub extern "C" fn sentil_bounds_clamp(handle: *mut c_void, point: *mut f64, n: size_t) {
    clear_error();
    ffi_panic_boundary((), || {
        if handle.is_null() || point.is_null() {
            set_error(SentilError::NullPointer, "a bounds or point argument was null");
            return;
        }
        let bounds = unsafe { &*handle.cast::<Bounds>() };
        let slice = unsafe { std::slice::from_raw_parts_mut(point, n) };
        bounds.clamp(slice);
    });
}

#[no_mangle]
pub extern "C" fn sentil_bounds_dimension(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, Bounds, 0).dimension())
}

fn copy_bounds_limit(handle: *mut c_void, out: *mut f64, upper: bool) {
    if handle.is_null() || out.is_null() {
        set_error(SentilError::NullPointer, "a bounds or output argument was null");
        return;
    }
    let bounds = unsafe { &*handle.cast::<Bounds>() };
    let limits = if upper { bounds.upper() } else { bounds.lower() };
    unsafe { std::ptr::copy_nonoverlapping(limits.as_ptr(), out, limits.len()) };
}

#[no_mangle]
pub extern "C" fn sentil_bounds_lower(handle: *mut c_void, out: *mut f64) {
    clear_error();
    ffi_panic_boundary((), || copy_bounds_limit(handle, out, false));
}

#[no_mangle]
pub extern "C" fn sentil_bounds_upper(handle: *mut c_void, out: *mut f64) {
    clear_error();
    ffi_panic_boundary((), || copy_bounds_limit(handle, out, true));
}

#[no_mangle]
pub extern "C" fn sentil_bounds_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Bounds>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_linear_model_create(
    a: *const f64,
    n: size_t,
    b: *const f64,
    b_cols: size_t,
    x0: *const f64,
    variables: *const *const c_char,
    n_vars: size_t,
    dt: f64,
    horizon: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || {
        let a = match matrix_from(a, n, n) {
            Ok(v) => v,
            Err(_) => return std::ptr::null_mut(),
        };
        let b = match matrix_from(b, n, b_cols) {
            Ok(v) => v,
            Err(_) => return std::ptr::null_mut(),
        };
        let Ok(x0) = slice_from(x0, n) else {
            return std::ptr::null_mut();
        };
        let variables = match collect_strings(variables, n_vars) {
            Ok(v) => v,
            Err(_) => return std::ptr::null_mut(),
        };
        match LinearModel::new(a, b, x0.to_vec(), variables, dt, horizon) {
            Ok(model) => into_handle(Box::new(model) as ModelHandle),
            Err(e) => {
                let _: SentilError = e.into();
                std::ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_system_model_create_custom(
    variables: *const *const c_char,
    n_vars: size_t,
    dt: f64,
    horizon: size_t,
    vtable: SentilModelVtable,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || {
        let variables = match collect_strings(variables, n_vars) {
            Ok(v) => v,
            Err(_) => return std::ptr::null_mut(),
        };
        let Some(rollout) = vtable.rollout else {
            set_error(SentilError::NullPointer, "the rollout callback was null");
            return std::ptr::null_mut();
        };
        let Ok(initial) = slice_from(vtable.initial_state, n_vars) else {
            return std::ptr::null_mut();
        };
        let model: ModelHandle = Box::new(CustomModel {
            variables,
            dt,
            horizon,
            initial: initial.to_vec(),
            input_dim: vtable.input_dimension,
            userdata: vtable.userdata as usize,
            rollout,
        });
        into_handle(model)
    })
}

#[no_mangle]
pub extern "C" fn sentil_system_model_input_dimension(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, ModelHandle, 0).input_dimension())
}

#[no_mangle]
pub extern "C" fn sentil_system_model_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<ModelHandle>(handle) });
}

/// The synthesized input and how well it does.
#[repr(C)]
pub struct SentilSynthesisResult {
    pub input: *mut f64,
    pub input_len: size_t,
    pub robustness: f64,
    pub holds: bool,
    pub backend: SentilBackend,
}

#[no_mangle]
pub extern "C" fn sentil_synthesize(
    model: *mut c_void,
    spec: *mut c_void,
    bounds: *mut c_void,
    smooth: *const SentilSmoothConfig,
    max_iters: size_t,
    backend: SentilBackend,
    population: size_t,
    out: *mut SentilSynthesisResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let model = borrow_handle!(model, ModelHandle, SentilError::NullPointer);
        let spec = borrow_handle!(spec, Formula, SentilError::NullPointer);
        let model = DynModel(&**model);
        let mut problem = SynthesisProblem::new(&model, spec).with_backend(backend.into());
        if max_iters != 0 {
            problem = problem.with_budget(max_iters);
        }
        if population != 0 {
            problem = problem.with_population(population);
        }
        if !bounds.is_null() {
            let b = unsafe { &*bounds.cast::<Bounds>() };
            let rebuilt = match Bounds::new(b.lower().to_vec(), b.upper().to_vec()) {
                Ok(x) => x,
                Err(e) => return e.into(),
            };
            problem = problem.with_bounds(rebuilt);
        }
        if let Some(smooth) = unsafe { smooth.as_ref() } {
            let smooth = match smooth.to_core() {
                Ok(x) => x,
                Err(e) => return e.into(),
            };
            problem = problem.with_smooth(smooth);
        }
        match Synthesizer::solve(&problem) {
            Ok(result) => {
                let mut input_len = 0;
                let input = into_boxed_array(result.input, &mut input_len);
                unsafe {
                    *out = SentilSynthesisResult {
                        input,
                        input_len,
                        robustness: result.robustness,
                        holds: result.holds,
                        backend: result.backend.into(),
                    };
                }
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

/// A gradient objective.
pub type SentilGradientFn = unsafe extern "C" fn(*mut c_void, *const f64, size_t, *mut f64, *mut f64);
/// A scalar objective for the point `x`.
pub type SentilObjectiveFn = unsafe extern "C" fn(*mut c_void, *const f64, size_t) -> f64;

fn write_optimum(
    result: sentil::Result<(Vec<f64>, f64)>,
    out_point: *mut f64,
    out_value: *mut f64,
) -> SentilError {
    match result {
        Ok((point, value)) => {
            unsafe {
                std::ptr::copy_nonoverlapping(point.as_ptr(), out_point, point.len());
                *out_value = value;
            }
            SentilError::Ok
        }
        Err(e) => e.into(),
    }
}

/// CMA-ES settings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilCmaConfig {
    pub population: size_t,
    pub max_generations: size_t,
    pub initial_step: f64,
    pub tol_step: f64,
    pub seed: u64,
}

impl From<SentilCmaConfig> for CmaConfig {
    fn from(c: SentilCmaConfig) -> Self {
        CmaConfig {
            population: c.population,
            max_generations: c.max_generations,
            initial_step: c.initial_step,
            tol_step: c.tol_step,
            seed: c.seed,
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_cma_config_default() -> SentilCmaConfig {
    let d = CmaConfig::default();
    SentilCmaConfig {
        population: d.population,
        max_generations: d.max_generations,
        initial_step: d.initial_step,
        tol_step: d.tol_step,
        seed: d.seed,
    }
}

#[no_mangle]
pub extern "C" fn sentil_maximize(
    objective: Option<SentilGradientFn>,
    userdata: *mut c_void,
    start: *const f64,
    n: size_t,
    bounds: *mut c_void,
    max_iters: size_t,
    out_point: *mut f64,
    out_value: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out_point, SentilError::NullPointer);
        check_ptr!(out_value, SentilError::NullPointer);
        let Some(objective) = objective else {
            set_error(SentilError::NullPointer, "the objective callback was null");
            return SentilError::NullPointer;
        };
        let Ok(start) = slice_from(start, n) else {
            return SentilError::NullPointer;
        };
        let bounds = borrow_handle!(bounds, Bounds, SentilError::NullPointer);
        let result = maximize(
            |x: &[f64]| {
                let mut value = 0.0_f64;
                let mut gradient = vec![0.0_f64; x.len()];
                unsafe {
                    objective(userdata, x.as_ptr(), x.len(), &mut value, gradient.as_mut_ptr());
                }
                Ok((value, gradient))
            },
            start,
            bounds,
            max_iters,
        );
        write_optimum(result, out_point, out_value)
    })
}

#[no_mangle]
pub extern "C" fn sentil_cma_es(
    objective: Option<SentilObjectiveFn>,
    userdata: *mut c_void,
    start: *const f64,
    n: size_t,
    bounds: *mut c_void,
    config: SentilCmaConfig,
    out_point: *mut f64,
    out_value: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out_point, SentilError::NullPointer);
        check_ptr!(out_value, SentilError::NullPointer);
        let Some(objective) = objective else {
            set_error(SentilError::NullPointer, "the objective callback was null");
            return SentilError::NullPointer;
        };
        let Ok(start) = slice_from(start, n) else {
            return SentilError::NullPointer;
        };
        let bounds = borrow_handle!(bounds, Bounds, SentilError::NullPointer);
        let result = cma_es(
            |x: &[f64]| Ok(unsafe { objective(userdata, x.as_ptr(), x.len()) }),
            start,
            bounds,
            config.into(),
        );
        write_optimum(result, out_point, out_value)
    })
}

/// A batch objective.
pub type SentilBatchObjectiveFn =
    unsafe extern "C" fn(*mut c_void, *const f64, size_t, size_t, *mut f64);

#[no_mangle]
pub extern "C" fn sentil_cma_es_batched(
    objective: Option<SentilBatchObjectiveFn>,
    userdata: *mut c_void,
    start: *const f64,
    n: size_t,
    bounds: *mut c_void,
    config: SentilCmaConfig,
    out_point: *mut f64,
    out_value: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out_point, SentilError::NullPointer);
        check_ptr!(out_value, SentilError::NullPointer);
        let Some(objective) = objective else {
            set_error(SentilError::NullPointer, "the objective callback was null");
            return SentilError::NullPointer;
        };
        let Ok(start) = slice_from(start, n) else {
            return SentilError::NullPointer;
        };
        let bounds = borrow_handle!(bounds, Bounds, SentilError::NullPointer);
        let result = cma_es_batched(
            |points: &[Vec<f64>]| {
                let population = points.len();
                let dim = points.first().map_or(0, Vec::len);
                let mut flat = Vec::with_capacity(population * dim);
                for point in points {
                    flat.extend_from_slice(point);
                }
                let mut scores = vec![0.0_f64; population];
                unsafe {
                    objective(userdata, flat.as_ptr(), population, dim, scores.as_mut_ptr());
                }
                Ok(scores)
            },
            start,
            bounds,
            config.into(),
        );
        write_optimum(result, out_point, out_value)
    })
}

struct ControllerState {
    controller: Controller<'static, DynModel<'static>>,
    model: *mut DynModel<'static>,
    spec: *mut Formula,
    owned: *mut dyn SystemModel,
}

#[no_mangle]
pub extern "C" fn sentil_controller_create(
    model: *mut c_void,
    spec: *mut c_void,
    input_width: size_t,
    budget_ns: u64,
    bounds: *mut c_void,
    smooth: *const SentilSmoothConfig,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || {
        let (Some(owned), Some(spec)) =
            (unsafe { take_handle::<ModelHandle>(model) }, unsafe { take_handle::<Formula>(spec) })
        else {
            set_error(SentilError::NullPointer, "the model or spec handle was null");
            return std::ptr::null_mut();
        };
        let bounds = if bounds.is_null() {
            None
        } else {
            let b = unsafe { &*bounds.cast::<Bounds>() };
            match Bounds::new(b.lower().to_vec(), b.upper().to_vec()) {
                Ok(rebuilt) => Some(rebuilt),
                Err(e) => {
                    let _: SentilError = e.into();
                    return std::ptr::null_mut();
                }
            }
        };
        let smooth = match unsafe { smooth.as_ref() } {
            Some(s) => match s.to_core() {
                Ok(c) => Some(c),
                Err(e) => {
                    let _: SentilError = e.into();
                    return std::ptr::null_mut();
                }
            },
            None => None,
        };
        let owned = Box::into_raw(owned);
        let model = Box::into_raw(Box::new(DynModel(unsafe { &*owned })));
        let spec = Box::into_raw(Box::new(spec));
        let mut controller = Controller::new(
            unsafe { &*model },
            unsafe { &*spec },
            input_width,
            Duration::from_nanos(budget_ns),
        );
        if let Some(b) = bounds {
            controller = controller.with_bounds(b);
        }
        if let Some(s) = smooth {
            controller = controller.with_smooth(s);
        }
        into_handle(ControllerState { controller, model, spec, owned })
    })
}

#[no_mangle]
pub extern "C" fn sentil_controller_control(
    handle: *mut c_void,
    state: *const f64,
    n: size_t,
    out: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let controller = borrow_handle_mut!(handle, ControllerState, SentilError::NullPointer);
        let state = match slice_from(state, n) {
            Ok(s) => s,
            Err(code) => return code,
        };
        match controller.controller.control(state) {
            Ok(input) => {
                unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), out, input.len()) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_controller_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || {
        if handle.is_null() {
            return;
        }
        let ControllerState { controller, model, spec, owned } =
            unsafe { *Box::from_raw(handle.cast::<ControllerState>()) };
        drop(controller);
        unsafe {
            drop(Box::from_raw(model));
            drop(Box::from_raw(spec));
            drop(Box::from_raw(owned));
        }
    });
}

#[no_mangle]
pub extern "C" fn sentil_safety_filter_create(bounds: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || {
        let Some(bounds) = (unsafe { take_handle::<Bounds>(bounds) }) else {
            set_error(SentilError::NullPointer, "the bounds handle was null");
            return std::ptr::null_mut();
        };
        into_handle(SafetyFilter::new(bounds))
    })
}

#[no_mangle]
pub extern "C" fn sentil_safety_filter_filter(
    handle: *mut c_void,
    nominal: *const f64,
    n: size_t,
    barrier_a: *const f64,
    barrier_b: *const f64,
    m: size_t,
    out: *mut f64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let filter = borrow_handle!(handle, SafetyFilter, SentilError::NullPointer);
        let nominal = match slice_from(nominal, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let rows = match matrix_from(barrier_a, m, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let scalars = match slice_from(barrier_b, m) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let barriers: Vec<(Vec<f64>, f64)> =
            rows.into_iter().zip(scalars).map(|(a, &b)| (a, b)).collect();
        match filter.filter(nominal, &barriers) {
            Ok(input) => {
                unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), out, input.len()) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_safety_filter_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<SafetyFilter>(handle) });
}

/// A chance-constraint report.
#[repr(C)]
pub struct SentilChanceReport {
    pub estimate: f64,
    pub lower_bound: f64,
    pub samples: u64,
    pub holds: bool,
}

impl From<ChanceReport> for SentilChanceReport {
    fn from(r: ChanceReport) -> Self {
        Self { estimate: r.estimate, lower_bound: r.lower_bound, samples: r.samples, holds: r.holds }
    }
}

#[no_mangle]
pub extern "C" fn sentil_chance_constraint_create(
    spec: *mut c_void,
    probability: f64,
    confidence: f64,
    tightening: f64,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(std::ptr::null_mut(), || {
        let Some(spec) = (unsafe { take_handle::<Formula>(spec) }) else {
            set_error(SentilError::NullPointer, "the spec handle was null");
            return std::ptr::null_mut();
        };
        let mut constraint = match ChanceConstraint::new(spec, probability) {
            Ok(c) => c,
            Err(e) => {
                let _: SentilError = e.into();
                return std::ptr::null_mut();
            }
        };
        if confidence > 0.0 {
            constraint = constraint.with_confidence(confidence);
        }
        if tightening != 0.0 {
            constraint = constraint.with_tightening(tightening);
        }
        into_handle(constraint)
    })
}

#[no_mangle]
pub extern "C" fn sentil_chance_constraint_validate(
    handle: *mut c_void,
    system: *mut c_void,
    samples: u64,
    seed: u64,
    out: *mut SentilChanceReport,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let constraint = borrow_handle!(handle, ChanceConstraint, SentilError::NullPointer);
        let system = borrow_handle!(system, StochasticSystem, SentilError::NullPointer);
        match constraint.validate(system, samples, seed) {
            Ok(report) => {
                unsafe { *out = report.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_chance_constraint_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<ChanceConstraint>(handle) });
}