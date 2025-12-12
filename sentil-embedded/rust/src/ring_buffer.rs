//! A fixed-size rolling window with running statistics.

use alloc::boxed::Box;
use sentil::RingBuffer;

use crate::{status_of, Status};

/// A timestamped sample, returned by the buffer's read calls.
#[repr(C)]
pub struct Sample {
    /// Timestamp the sample was recorded at.
    pub time: f64,
    /// The reading itself.
    pub value: f64,
}

/// Creates a ring buffer that holds the most recent `capacity` samples.
///
/// # Safety
///
/// `out` must point to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_create(
    capacity: usize,
    out: *mut *mut RingBuffer,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    match RingBuffer::new(capacity) {
        Ok(buffer) => {
            *out = Box::into_raw(Box::new(buffer));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Pushes a sample, evicting the oldest when the buffer is full.
///
/// # Safety
///
/// `buffer` must be a live handle; the out-pointers, if non-null, must be writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_push(
    buffer: *mut RingBuffer,
    time: f64,
    value: f64,
    out_evicted: *mut Sample,
    out_did_evict: *mut bool,
) -> Status {
    if buffer.is_null() {
        return Status::NullPointer;
    }
    match (*buffer).push(time, value) {
        Ok(evicted) => {
            let did = evicted.is_some();
            if let Some((time, value)) = evicted {
                if !out_evicted.is_null() {
                    *out_evicted = Sample { time, value };
                }
            }
            if !out_did_evict.is_null() {
                *out_did_evict = did;
            }
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

macro_rules! count_accessor {
    ($(#[$doc:meta])* $name:ident, $method:ident) => {
        $(#[$doc])*
        ///
        /// # Safety
        ///
        /// `buffer` must be a live handle or null.
        #[no_mangle]
        pub unsafe extern "C" fn $name(buffer: *const RingBuffer) -> usize {
            if buffer.is_null() { 0 } else { (*buffer).$method() }
        }
    };
}

count_accessor!(
    /// The number of samples currently held.
    sentil_embedded_ring_buffer_len, len);
count_accessor!(
    /// The fixed capacity of the buffer.
    sentil_embedded_ring_buffer_capacity, capacity);

macro_rules! flag_accessor {
    ($(#[$doc:meta])* $name:ident, $method:ident) => {
        $(#[$doc])*
        ///
        /// # Safety
        ///
        /// `buffer` must be a live handle or null.
        #[no_mangle]
        pub unsafe extern "C" fn $name(buffer: *const RingBuffer) -> bool {
            !buffer.is_null() && (*buffer).$method()
        }
    };
}

flag_accessor!(
    /// Whether the buffer holds no samples.
    sentil_embedded_ring_buffer_is_empty, is_empty);
flag_accessor!(
    /// Whether the buffer is at capacity.
    sentil_embedded_ring_buffer_is_full, is_full);

macro_rules! stat_accessor {
    ($(#[$doc:meta])* $name:ident, $method:ident) => {
        $(#[$doc])*
        ///
        /// # Safety
        ///
        /// `buffer` must be a live handle or null.
        #[no_mangle]
        pub unsafe extern "C" fn $name(buffer: *const RingBuffer) -> f64 {
            if buffer.is_null() { f64::NAN } else { (*buffer).$method().unwrap_or(f64::NAN) }
        }
    };
}

stat_accessor!(
    /// The running mean of the held values, or NaN when empty.
    sentil_embedded_ring_buffer_mean, mean);
stat_accessor!(
    /// The running variance of the held values, or NaN when empty.
    sentil_embedded_ring_buffer_variance, variance);
stat_accessor!(
    /// The running standard deviation, or NaN when empty.
    sentil_embedded_ring_buffer_std_dev, std_dev);
stat_accessor!(
    /// The smallest held value, or NaN when empty.
    sentil_embedded_ring_buffer_min, min);
stat_accessor!(
    /// The largest held value, or NaN when empty.
    sentil_embedded_ring_buffer_max, max);

macro_rules! sample_accessor {
    ($(#[$doc:meta])* $name:ident, $method:ident) => {
        $(#[$doc])*
        ///
        /// # Safety
        ///
        /// `buffer` must be a live handle and `out` writable.
        #[no_mangle]
        pub unsafe extern "C" fn $name(buffer: *const RingBuffer, out: *mut Sample) -> bool {
            if buffer.is_null() || out.is_null() { return false; }
            match (*buffer).$method() {
                Some((time, value)) => { *out = Sample { time, value }; true }
                None => false,
            }
        }
    };
}

sample_accessor!(
    /// The oldest sample, written to `out`; returns false when empty.
    sentil_embedded_ring_buffer_front, front);
sample_accessor!(
    /// The newest sample, written to `out`; returns false when empty.
    sentil_embedded_ring_buffer_back, back);

/// The sample at `index` from the oldest, written to `out`.
///
/// # Safety
///
/// `buffer` must be a live handle and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_get(
    buffer: *const RingBuffer,
    index: usize,
    out: *mut Sample,
) -> bool {
    if buffer.is_null() || out.is_null() {
        return false;
    }
    match (*buffer).get(index) {
        Some((time, value)) => {
            *out = Sample { time, value };
            true
        }
        None => false,
    }
}

/// The value recorded at `time`, within a small tolerance, or NaN.
///
/// # Safety
///
/// `buffer` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_at_time(buffer: *const RingBuffer, time: f64) -> f64 {
    if buffer.is_null() {
        return f64::NAN;
    }
    (*buffer).at_time(time).unwrap_or(f64::NAN)
}

/// The sample whose time is closest to `time`, written to `out`.
///
/// # Safety
///
/// `buffer` must be a live handle and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_closest_to_time(
    buffer: *const RingBuffer,
    time: f64,
    out: *mut Sample,
) -> bool {
    if buffer.is_null() || out.is_null() {
        return false;
    }
    match (*buffer).closest_to_time(time) {
        Some((time, value)) => {
            *out = Sample { time, value };
            true
        }
        None => false,
    }
}

/// Empties the buffer. A null pointer is a no-op.
///
/// # Safety
///
/// `buffer` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_clear(buffer: *mut RingBuffer) {
    if !buffer.is_null() {
        (*buffer).clear();
    }
}

/// Frees a ring buffer. A null pointer is a no-op.
///
/// # Safety
///
/// `buffer` must be a live handle that has not been destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_ring_buffer_destroy(buffer: *mut RingBuffer) {
    if !buffer.is_null() {
        drop(Box::from_raw(buffer));
    }
}