use crate::conversions::{
    c_char_to_string, clear_error, ffi_panic_boundary, into_string_array, set_error, slice_from,
};
use crate::handles::{drop_handle, into_handle};
use crate::{SentilError, SentilInterpolation};
use libc::{c_char, c_double, c_void, size_t};
use sentil::{RingBuffer, Trace};
use std::ptr;

/// A timed sample.
#[repr(C)]
pub struct SentilSample {
    pub found: bool,
    pub time: f64,
    pub value: f64,
}

impl SentilSample {
    fn present(time: f64, value: f64) -> Self {
        Self { found: true, time, value }
    }

    fn absent() -> Self {
        Self { found: false, time: 0.0, value: 0.0 }
    }

    fn from_pair(pair: Option<(f64, f64)>) -> Self {
        pair.map_or_else(Self::absent, |(t, v)| Self::present(t, v))
    }
}

#[no_mangle]
pub extern "C" fn sentil_trace_create(times: *const c_double, n: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        match Trace::new(times.to_vec()) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_signal(
    times: *const c_double,
    n: size_t,
    name: *const c_char,
    values: *const c_double,
    m: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        let Ok(values) = slice_from(values, m) else {
            return ptr::null_mut();
        };
        match Trace::from_signal(times.to_vec(), &name, values.to_vec()) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_indexed(len: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(Trace::indexed(len)))
}

#[no_mangle]
pub extern "C" fn sentil_trace_add_signal(
    handle: *mut c_void,
    name: *const c_char,
    values: *const c_double,
    n: size_t,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let trace = borrow_handle_mut!(handle, Trace, SentilError::NullPointer);
        let name = match c_char_to_string(name) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let values = match slice_from(values, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        match trace.add_signal(&name, values.to_vec()) {
            Ok(()) => SentilError::Ok,
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_len(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, Trace, 0).len())
}

#[no_mangle]
pub extern "C" fn sentil_trace_is_empty(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(true, || borrow_handle!(handle, Trace, true).is_empty())
}

#[no_mangle]
pub extern "C" fn sentil_trace_times(handle: *mut c_void, out_len: *mut size_t) -> *const c_double {
    clear_error();
    ffi_panic_boundary(ptr::null(), || {
        check_ptr!(out_len, ptr::null());
        let trace = borrow_handle!(handle, Trace, ptr::null());
        let times = trace.times();
        unsafe { *out_len = times.len() };
        times.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_variables(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let trace = borrow_handle!(handle, Trace, ptr::null_mut());
        let names = trace.variables().into_iter().map(String::from).collect();
        into_string_array(names, out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_signal(
    handle: *mut c_void,
    name: *const c_char,
    out_len: *mut size_t,
) -> *const c_double {
    clear_error();
    ffi_panic_boundary(ptr::null(), || {
        check_ptr!(out_len, ptr::null());
        let trace = borrow_handle!(handle, Trace, ptr::null());
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null();
        };
        match trace.signal(&name) {
            Some(values) => {
                unsafe { *out_len = values.len() };
                values.as_ptr()
            }
            None => {
                unsafe { *out_len = 0 };
                set_error(SentilError::UnknownVariable, &format!("trace has no signal named `{name}`"));
                ptr::null()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_resample(
    handle: *mut c_void,
    times: *const c_double,
    n: size_t,
    interp: SentilInterpolation,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let trace = borrow_handle!(handle, Trace, ptr::null_mut());
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        match trace.resample(times.to_vec(), interp.into()) {
            Ok(resampled) => into_handle(resampled),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

fn trace_from_text(text: *const c_char, parse: fn(&str) -> sentil::Result<Trace>) -> *mut c_void {
    let Ok(text) = c_char_to_string(text) else {
        return ptr::null_mut();
    };
    match parse(&text) {
        Ok(trace) => into_handle(trace),
        Err(e) => {
            let _: SentilError = e.into();
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_csv(text: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || trace_from_text(text, Trace::from_csv_str))
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_tsv(text: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || trace_from_text(text, Trace::from_tsv_str))
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_path(path: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(path) = c_char_to_string(path) else {
            return ptr::null_mut();
        };
        match Trace::from_path(&path) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Trace>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_create(capacity: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || match RingBuffer::new(capacity) {
        Ok(buffer) => into_handle(buffer),
        Err(e) => {
            let _: SentilError = e.into();
            ptr::null_mut()
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_push(
    handle: *mut c_void,
    time: c_double,
    value: c_double,
    out_evicted: *mut SentilSample,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let buffer = borrow_handle_mut!(handle, RingBuffer, SentilError::NullPointer);
        match buffer.push(time, value) {
            Ok(evicted) => {
                if let Some(slot) = unsafe { out_evicted.as_mut() } {
                    *slot = SentilSample::from_pair(evicted);
                }
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_clear(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { mutate_handle(handle, RingBuffer::clear) });
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_len(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, RingBuffer, 0).len())
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_capacity(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, RingBuffer, 0).capacity())
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_is_empty(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(true, || borrow_handle!(handle, RingBuffer, true).is_empty())
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_is_full(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(false, || borrow_handle!(handle, RingBuffer, false).is_full())
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_front(handle: *mut c_void) -> SentilSample {
    clear_error();
    ffi_panic_boundary(SentilSample::absent(), || {
        SentilSample::from_pair(borrow_handle!(handle, RingBuffer, SentilSample::absent()).front())
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_back(handle: *mut c_void) -> SentilSample {
    clear_error();
    ffi_panic_boundary(SentilSample::absent(), || {
        SentilSample::from_pair(borrow_handle!(handle, RingBuffer, SentilSample::absent()).back())
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_get(handle: *mut c_void, index: size_t) -> SentilSample {
    clear_error();
    ffi_panic_boundary(SentilSample::absent(), || {
        let buffer = borrow_handle!(handle, RingBuffer, SentilSample::absent());
        SentilSample::from_pair(buffer.get(index))
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_pop_front(handle: *mut c_void) -> SentilSample {
    clear_error();
    ffi_panic_boundary(SentilSample::absent(), || {
        let buffer = borrow_handle_mut!(handle, RingBuffer, SentilSample::absent());
        SentilSample::from_pair(buffer.pop_front())
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_pop_back(handle: *mut c_void) -> SentilSample {
    clear_error();
    ffi_panic_boundary(SentilSample::absent(), || {
        let buffer = borrow_handle_mut!(handle, RingBuffer, SentilSample::absent());
        SentilSample::from_pair(buffer.pop_back())
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_closest_to_time(handle: *mut c_void, time: c_double) -> SentilSample {
    clear_error();
    ffi_panic_boundary(SentilSample::absent(), || {
        let buffer = borrow_handle!(handle, RingBuffer, SentilSample::absent());
        SentilSample::from_pair(buffer.closest_to_time(time))
    })
}

fn optional_stat(
    handle: *mut c_void,
    get: impl FnOnce(&RingBuffer) -> Option<f64>,
    out: *mut c_double,
) -> bool {
    check_ptr!(out, false);
    let buffer = borrow_handle!(handle, RingBuffer, false);
    match get(buffer) {
        Some(v) => {
            unsafe { *out = v };
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_mean(handle: *mut c_void, out: *mut c_double) -> bool {
    clear_error();
    ffi_panic_boundary(false, || optional_stat(handle, RingBuffer::mean, out))
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_variance(handle: *mut c_void, out: *mut c_double) -> bool {
    clear_error();
    ffi_panic_boundary(false, || optional_stat(handle, RingBuffer::variance, out))
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_std_dev(handle: *mut c_void, out: *mut c_double) -> bool {
    clear_error();
    ffi_panic_boundary(false, || optional_stat(handle, RingBuffer::std_dev, out))
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_min(handle: *mut c_void, out: *mut c_double) -> bool {
    clear_error();
    ffi_panic_boundary(false, || optional_stat(handle, RingBuffer::min, out))
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_max(handle: *mut c_void, out: *mut c_double) -> bool {
    clear_error();
    ffi_panic_boundary(false, || optional_stat(handle, RingBuffer::max, out))
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_recompute_statistics(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { mutate_handle(handle, RingBuffer::recompute_statistics) });
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_at_time(
    handle: *mut c_void,
    time: c_double,
    out: *mut c_double,
) -> bool {
    clear_error();
    ffi_panic_boundary(false, || optional_stat(handle, |b| b.at_time(time), out))
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_time_range(
    handle: *mut c_void,
    out_start: *mut c_double,
    out_end: *mut c_double,
) -> bool {
    clear_error();
    ffi_panic_boundary(false, || {
        check_ptr!(out_start, false);
        check_ptr!(out_end, false);
        let buffer = borrow_handle!(handle, RingBuffer, false);
        match buffer.time_range() {
            Some((start, end)) => {
                unsafe {
                    *out_start = start;
                    *out_end = end;
                }
                true
            }
            None => false,
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_between(
    handle: *mut c_void,
    start: c_double,
    end: c_double,
    out_count: *mut size_t,
) -> *mut SentilSample {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let buffer = borrow_handle!(handle, RingBuffer, ptr::null_mut());
        let samples: Vec<SentilSample> =
            buffer.between(start, end).map(|(t, v)| SentilSample::present(t, v)).collect();
        let len = samples.len();
        let raw = Box::into_raw(samples.into_boxed_slice());
        unsafe { *out_count = len };
        raw.cast::<SentilSample>()
    })
}

#[no_mangle]
pub extern "C" fn sentil_free_samples(samples: *mut SentilSample, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || {
        if samples.is_null() {
            return;
        }
        unsafe { drop(Box::from_raw(ptr::slice_from_raw_parts_mut(samples, count))) };
    });
}

#[no_mangle]
pub extern "C" fn sentil_ring_buffer_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<RingBuffer>(handle) });
}