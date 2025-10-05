use crate::conversions::{
    c_char_to_string, clear_error, code_of, ffi_panic_boundary, into_string_array, set_error,
    slice_from, to_c_string,
};
use crate::stats::SentilSmcConfig;
use crate::{SentilError, SentilTimeMode};
use libc::{c_char, c_double, c_void, size_t};
use sentil::{
    Formula, FormulaBank, LiftingRegistry, Monitor, MonitorConfig, MultiFormulaMonitor, SmcConfig,
    StreamMonitor, Trace,
};
use std::ptr;

/// A time span [start, end] where a property does not hold.
#[repr(C)]
pub struct SentilInterval {
    pub start: f64,
    pub end: f64,
}

fn pack_intervals(spans: Vec<(f64, f64)>, out_count: *mut size_t) -> *mut SentilInterval {
    let intervals = spans.into_iter().map(|(start, end)| SentilInterval { start, end }).collect();
    into_boxed_array(intervals, out_count)
}

/// A robustness verdict.
#[repr(C)]
pub struct SentilRobustness {
    pub resolved: bool,
    pub satisfied: bool,
    pub value: f64,
    pub lower: f64,
    pub upper: f64,
}

impl SentilRobustness {
    fn from_core(r: sentil::Robustness) -> Self {
        Self {
            resolved: r.is_resolved(),
            satisfied: r.is_satisfied(),
            value: r.value(),
            lower: r.lower(),
            upper: r.upper(),
        }
    }
}

fn collect_named(
    names: *const *const c_char,
    values: *const c_double,
    n: size_t,
) -> Result<Vec<(String, f64)>, SentilError> {
    let names = slice_from(names, n)?;
    let values = slice_from(values, n)?;
    let mut pairs = Vec::with_capacity(n);
    for (&name, &value) in names.iter().zip(values) {
        pairs.push((c_char_to_string(name)?, value));
    }
    Ok(pairs)
}

fn config_or_default(config: *mut c_void) -> MonitorConfig {
    if config.is_null() {
        MonitorConfig::default()
    } else {
        unsafe { (*config.cast::<MonitorConfig>()).clone() }
    }
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_create() -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(MonitorConfig::new()))
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_set_time(
    handle: *mut c_void,
    mode: SentilTimeMode,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let config = borrow_handle_mut!(handle, MonitorConfig, SentilError::NullPointer);
        *config = std::mem::take(config).time(mode.into());
        SentilError::Ok
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_time_mode(handle: *mut c_void) -> SentilTimeMode {
    clear_error();
    ffi_panic_boundary(SentilTimeMode::Discrete, || {
        borrow_handle!(handle, MonitorConfig, SentilTimeMode::Discrete).time_mode().into()
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<MonitorConfig>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_monitor_create(formula: *mut c_void, config: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Some(formula) = (unsafe { take_handle::<Formula>(formula) }) else {
            set_error(SentilError::NullPointer, "the formula handle was null");
            return ptr::null_mut();
        };
        into_handle(Monitor::from_formula(formula, config_or_default(config)))
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_parse(formula: *const c_char, config: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(formula) else {
            return ptr::null_mut();
        };
        match Monitor::new(&text, config_or_default(config)) {
            Ok(monitor) => into_handle(monitor),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_formula(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let monitor = borrow_handle!(handle, Monitor, ptr::null_mut());
        into_handle(monitor.formula().clone())
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let monitor = borrow_handle!(handle, Monitor, ptr::null_mut());
        into_handle(monitor.config().clone())
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_update(
    handle: *mut c_void,
    time: c_double,
    names: *const *const c_char,
    values: *const c_double,
    n: size_t,
    out: *mut SentilRobustness,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle_mut!(handle, Monitor, SentilError::NullPointer);
        let pairs = match collect_named(names, values, n) {
            Ok(p) => p,
            Err(code) => return code,
        };
        let refs: Vec<(&str, f64)> = pairs.iter().map(|(name, v)| (name.as_str(), *v)).collect();
        match monitor.update(time, &refs) {
            Ok(robustness) => {
                unsafe { *out = SentilRobustness::from_core(robustness) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_update_packed(
    handle: *mut c_void,
    time: c_double,
    values: *const c_double,
    n: size_t,
    out: *mut SentilRobustness,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle_mut!(handle, Monitor, SentilError::NullPointer);
        let values = match slice_from(values, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        match monitor.update_packed(time, values) {
            Ok(robustness) => {
                unsafe { *out = SentilRobustness::from_core(robustness) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_robustness(
    handle: *mut c_void,
    trace: *mut c_void,
    out: *mut c_double,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle!(handle, Monitor, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        match monitor.robustness(trace) {
            Ok(value) => {
                unsafe { *out = value };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_robustness_signal(
    handle: *mut c_void,
    trace: *mut c_void,
    out_len: *mut size_t,
) -> *mut c_double {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_len, ptr::null_mut());
        let monitor = borrow_handle!(handle, Monitor, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        match monitor.robustness_signal(trace) {
            Ok(values) => into_boxed_array(values, out_len),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_violations(
    handle: *mut c_void,
    trace: *mut c_void,
    out_count: *mut size_t,
) -> *mut SentilInterval {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let monitor = borrow_handle!(handle, Monitor, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        match monitor.violations(trace) {
            Ok(spans) => pack_intervals(spans, out_count),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

fn formula_scalar(
    formula: *mut c_void,
    trace: *mut c_void,
    out: *mut c_double,
    eval: impl Fn(&Formula, &Trace) -> Result<f64, sentil::Error>,
) -> SentilError {
    check_ptr!(out, SentilError::NullPointer);
    let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
    let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
    match eval(formula, trace) {
        Ok(value) => {
            unsafe { *out = value };
            SentilError::Ok
        }
        Err(e) => e.into(),
    }
}

fn formula_signal(
    formula: *mut c_void,
    trace: *mut c_void,
    out_len: *mut size_t,
    eval: impl Fn(&Formula, &Trace) -> Result<Vec<f64>, sentil::Error>,
) -> *mut c_double {
    check_ptr!(out_len, ptr::null_mut());
    let formula = borrow_handle!(formula, Formula, ptr::null_mut());
    let trace = borrow_handle!(trace, Trace, ptr::null_mut());
    match eval(formula, trace) {
        Ok(values) => into_boxed_array(values, out_len),
        Err(e) => {
            let _: SentilError = e.into();
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_formula_robustness(
    formula: *mut c_void,
    trace: *mut c_void,
    out: *mut c_double,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        formula_scalar(formula, trace, out, |f, t| f.robustness(t))
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_robustness_dense(
    formula: *mut c_void,
    trace: *mut c_void,
    out: *mut c_double,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        formula_scalar(formula, trace, out, |f, t| f.robustness_dense(t))
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_robustness_signal(
    formula: *mut c_void,
    trace: *mut c_void,
    out_len: *mut size_t,
) -> *mut c_double {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        formula_signal(formula, trace, out_len, |f, t| f.robustness_signal(t))
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_robustness_dense_signal(
    formula: *mut c_void,
    trace: *mut c_void,
    out_len: *mut size_t,
) -> *mut c_double {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        formula_signal(formula, trace, out_len, |f, t| f.robustness_dense_signal(t))
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_violations(
    formula: *mut c_void,
    trace: *mut c_void,
    out_count: *mut size_t,
) -> *mut SentilInterval {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let formula = borrow_handle!(formula, Formula, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        match formula.violations(trace) {
            Ok(spans) => pack_intervals(spans, out_count),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_violation_intervals(
    times: *const c_double,
    n: size_t,
    signal: *const c_double,
    m: size_t,
    out_count: *mut size_t,
) -> *mut SentilInterval {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        let Ok(signal) = slice_from(signal, m) else {
            return ptr::null_mut();
        };
        pack_intervals(sentil::violation_intervals(times, signal), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_free_intervals(intervals: *mut SentilInterval, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || unsafe { free_boxed_array(intervals, count) });
}

#[no_mangle]
pub extern "C" fn sentil_monitor_symbol_index(
    handle: *mut c_void,
    name: *const c_char,
    out_index: *mut size_t,
    out_found: *mut bool,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out_index, SentilError::NullPointer);
        check_ptr!(out_found, SentilError::NullPointer);
        let monitor = borrow_handle_mut!(handle, Monitor, SentilError::NullPointer);
        let name = match c_char_to_string(name) {
            Ok(s) => s,
            Err(code) => return code,
        };
        match monitor.symbol_index(&name) {
            Ok(Some(index)) => {
                unsafe {
                    *out_index = index;
                    *out_found = true;
                }
                SentilError::Ok
            }
            Ok(None) => {
                unsafe { *out_found = false };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_reset(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { mutate_handle(handle, Monitor::reset) });
}

#[no_mangle]
pub extern "C" fn sentil_monitor_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Monitor>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_create(formula: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(formula) else {
            return ptr::null_mut();
        };
        match StreamMonitor::new(&text) {
            Ok(monitor) => into_handle(monitor),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_from_formula(formula: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let formula = borrow_handle!(formula, Formula, ptr::null_mut());
        match StreamMonitor::from_formula(formula) {
            Ok(monitor) => into_handle(monitor),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_with_lifting(
    formula: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSmcConfig,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let formula = borrow_handle!(formula, Formula, ptr::null_mut());
        let lifting = borrow_handle!(lifting, LiftingRegistry, ptr::null_mut());
        let Some(config) = (unsafe { config.as_ref() }) else {
            set_error(SentilError::NullPointer, "the smc config was null");
            return ptr::null_mut();
        };
        let config: SmcConfig = (*config).into();
        match StreamMonitor::with_lifting(formula, lifting, &config) {
            Ok(monitor) => into_handle(monitor),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_variable_count(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, StreamMonitor, 0).variable_count())
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_symbol_index(
    handle: *mut c_void,
    name: *const c_char,
    out_index: *mut size_t,
    out_found: *mut bool,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out_index, SentilError::NullPointer);
        check_ptr!(out_found, SentilError::NullPointer);
        let monitor = borrow_handle!(handle, StreamMonitor, SentilError::NullPointer);
        let name = match c_char_to_string(name) {
            Ok(s) => s,
            Err(code) => return code,
        };
        match monitor.symbol_index(&name) {
            Some(index) => {
                unsafe {
                    *out_index = index;
                    *out_found = true;
                }
                SentilError::Ok
            }
            None => {
                unsafe { *out_found = false };
                SentilError::Ok
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_update(
    handle: *mut c_void,
    time: c_double,
    names: *const *const c_char,
    values: *const c_double,
    n: size_t,
    out: *mut SentilRobustness,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle_mut!(handle, StreamMonitor, SentilError::NullPointer);
        let pairs = match collect_named(names, values, n) {
            Ok(p) => p,
            Err(code) => return code,
        };
        let refs: Vec<(&str, f64)> = pairs.iter().map(|(name, v)| (name.as_str(), *v)).collect();
        match monitor.update(time, &refs) {
            Ok(robustness) => {
                unsafe { *out = SentilRobustness::from_core(robustness) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_update_packed(
    handle: *mut c_void,
    time: c_double,
    values: *const c_double,
    n: size_t,
    out: *mut SentilRobustness,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle_mut!(handle, StreamMonitor, SentilError::NullPointer);
        let values = match slice_from(values, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        match monitor.update_packed(time, values) {
            Ok(robustness) => {
                unsafe { *out = SentilRobustness::from_core(robustness) };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_run(
    handle: *mut c_void,
    trace: *mut c_void,
    out_count: *mut size_t,
) -> *mut SentilRobustness {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let monitor = borrow_handle_mut!(handle, StreamMonitor, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        match monitor.run(trace) {
            Ok(steps) => {
                let values = steps.into_iter().map(SentilRobustness::from_core).collect();
                into_boxed_array(values, out_count)
            }
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_free_robustness(array: *mut SentilRobustness, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || unsafe { free_boxed_array(array, count) });
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_reset(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { mutate_handle(handle, StreamMonitor::reset) });
}

#[no_mangle]
pub extern "C" fn sentil_stream_monitor_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<StreamMonitor>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_create() -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(MultiFormulaMonitor::new()))
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_add(
    handle: *mut c_void,
    id: *const c_char,
    formula: *const c_char,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let monitor = borrow_handle_mut!(handle, MultiFormulaMonitor, SentilError::NullPointer);
        let id = match c_char_to_string(id) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let formula = match c_char_to_string(formula) {
            Ok(s) => s,
            Err(code) => return code,
        };
        match monitor.add(id, &formula) {
            Ok(()) => SentilError::Ok,
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_add_formula(
    handle: *mut c_void,
    id: *const c_char,
    formula: *mut c_void,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let monitor = borrow_handle_mut!(handle, MultiFormulaMonitor, SentilError::NullPointer);
        let id = match c_char_to_string(id) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        match monitor.add_formula(id, formula) {
            Ok(()) => SentilError::Ok,
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_add_probabilistic(
    handle: *mut c_void,
    id: *const c_char,
    formula: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSmcConfig,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        let monitor = borrow_handle_mut!(handle, MultiFormulaMonitor, SentilError::NullPointer);
        let id = match c_char_to_string(id) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
        let config: SmcConfig = unsafe { *config }.into();
        match monitor.add_probabilistic(id, formula, lifting, &config) {
            Ok(()) => SentilError::Ok,
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_remove(handle: *mut c_void, id: *const c_char) -> bool {
    clear_error();
    ffi_panic_boundary(false, || {
        let monitor = borrow_handle_mut!(handle, MultiFormulaMonitor, false);
        let Ok(id) = c_char_to_string(id) else {
            return false;
        };
        monitor.remove(&id)
    })
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_reset(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { mutate_handle(handle, MultiFormulaMonitor::reset) });
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_len(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, MultiFormulaMonitor, 0).len())
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_is_empty(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(true, || borrow_handle!(handle, MultiFormulaMonitor, true).is_empty())
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_ids(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let monitor = borrow_handle!(handle, MultiFormulaMonitor, ptr::null_mut());
        let ids = monitor.ids().map(String::from).collect();
        into_string_array(ids, out_count)
    })
}

/// A formula id paired with its verdict.
#[repr(C)]
pub struct SentilNamedRobustness {
    pub id: *mut c_char,
    pub robustness: SentilRobustness,
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_update(
    handle: *mut c_void,
    time: c_double,
    names: *const *const c_char,
    values: *const c_double,
    n: size_t,
    out_count: *mut size_t,
) -> *mut SentilNamedRobustness {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let monitor = borrow_handle_mut!(handle, MultiFormulaMonitor, ptr::null_mut());
        let pairs = match collect_named(names, values, n) {
            Ok(p) => p,
            Err(_) => return ptr::null_mut(),
        };
        let refs: Vec<(&str, f64)> = pairs.iter().map(|(name, v)| (name.as_str(), *v)).collect();
        match monitor.update(time, &refs) {
            Ok(results) => {
                let verdicts = results
                    .into_iter()
                    .map(|(id, robustness)| SentilNamedRobustness {
                        id: to_c_string(&id),
                        robustness: SentilRobustness::from_core(robustness),
                    })
                    .collect();
                into_boxed_array(verdicts, out_count)
            }
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_free_named_robustness(array: *mut SentilNamedRobustness, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || unsafe {
        free_boxed_array_owning(array, count, |verdict| verdict.id)
    });
}

#[no_mangle]
pub extern "C" fn sentil_multi_monitor_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<MultiFormulaMonitor>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_create() -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(FormulaBank::new()))
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_add(
    handle: *mut c_void,
    id: *const c_char,
    formula: *const c_char,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let bank = borrow_handle_mut!(handle, FormulaBank, SentilError::NullPointer);
        let id = match c_char_to_string(id) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let formula = match c_char_to_string(formula) {
            Ok(s) => s,
            Err(code) => return code,
        };
        match bank.add(id, &formula) {
            Ok(()) => SentilError::Ok,
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_add_formula(
    handle: *mut c_void,
    id: *const c_char,
    formula: *mut c_void,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let bank = borrow_handle_mut!(handle, FormulaBank, SentilError::NullPointer);
        let id = match c_char_to_string(id) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        bank.add_formula(id, formula);
        SentilError::Ok
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_ids(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let bank = borrow_handle!(handle, FormulaBank, ptr::null_mut());
        into_string_array(bank.ids().map(String::from).collect(), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_len(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, FormulaBank, 0).len())
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_is_empty(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(true, || borrow_handle!(handle, FormulaBank, true).is_empty())
}

/// Per-formula robustness.
#[repr(C)]
pub struct SentilBankResult {
    pub id: *mut c_char,
    pub ok: bool,
    pub value: f64,
    pub code: SentilError,
}

fn pack_bank(
    results: Vec<(String, sentil::Result<f64>)>,
    out_count: *mut size_t,
) -> *mut SentilBankResult {
    let packed = results
        .into_iter()
        .map(|(id, result)| match result {
            Ok(value) => SentilBankResult { id: to_c_string(&id), ok: true, value, code: SentilError::Ok },
            Err(e) => SentilBankResult {
                id: to_c_string(&id),
                ok: false,
                value: f64::NAN,
                code: code_of(&e),
            },
        })
        .collect();
    into_boxed_array(packed, out_count)
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_robustness(
    handle: *mut c_void,
    trace: *mut c_void,
    out_count: *mut size_t,
) -> *mut SentilBankResult {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let bank = borrow_handle!(handle, FormulaBank, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        pack_bank(bank.robustness(trace), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_robustness_dense(
    handle: *mut c_void,
    trace: *mut c_void,
    out_count: *mut size_t,
) -> *mut SentilBankResult {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let bank = borrow_handle!(handle, FormulaBank, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        pack_bank(bank.robustness_dense(trace), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_free_bank_results(array: *mut SentilBankResult, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || unsafe {
        free_boxed_array_owning(array, count, |result| result.id)
    });
}

#[no_mangle]
pub extern "C" fn sentil_formula_bank_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<FormulaBank>(handle) });
}