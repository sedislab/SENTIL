use crate::*;
use core::ptr;
use sentil::StreamMonitor;
use std::ffi::CString;

fn robustness() -> EmbeddedRobustness {
    EmbeddedRobustness { resolved: false, satisfied: false, value: 0.0, lower: 0.0, upper: 0.0 }
}

#[test]
fn monitor_lifecycle() {
    unsafe {
        let mut monitor: *mut StreamMonitor = ptr::null_mut();
        let formula = CString::new("x > 0").unwrap();
        assert_eq!(sentil_embedded_create(formula.as_ptr(), &mut monitor), Status::Ok);
        assert!(!monitor.is_null());

        let values = [5.0f64];
        let mut out = robustness();
        assert_eq!(sentil_embedded_update(monitor, 0.0, values.as_ptr(), 1, &mut out), Status::Ok);
        assert_eq!(out.value, 5.0);
        assert_eq!(sentil_embedded_variable_count(monitor), 1);

        let mut index = 0usize;
        let mut found = false;
        let name = CString::new("x").unwrap();
        sentil_embedded_symbol_index(monitor, name.as_ptr(), &mut index, &mut found);
        assert!(found && index == 0);

        sentil_embedded_reset(monitor);
        sentil_embedded_destroy(monitor);
    }
}

#[test]
fn compiled_formula_lifecycle() {
    unsafe {
        let blob = codec::encode(&sentil::Formula::parse("always[0, 3](x > 0)").unwrap());
        let mut monitor: *mut StreamMonitor = ptr::null_mut();
        assert_eq!(
            sentil_embedded_create_compiled(blob.as_ptr(), blob.len(), &mut monitor),
            Status::Ok
        );
        assert!(!monitor.is_null());
        let values = [1.0f64];
        let mut out = robustness();
        assert_eq!(sentil_embedded_update(monitor, 0.0, values.as_ptr(), 1, &mut out), Status::Ok);
        sentil_embedded_destroy(monitor);
    }
}

#[test]
fn bad_input_never_faults() {
    unsafe {
        let mut monitor: *mut StreamMonitor = ptr::null_mut();
        let bad = CString::new("always[0,").unwrap();
        assert_eq!(sentil_embedded_create(bad.as_ptr(), &mut monitor), Status::Parse);
        assert!(monitor.is_null());

        let junk = [1u8, 2, 3, 4];
        assert_eq!(
            sentil_embedded_create_compiled(junk.as_ptr(), junk.len(), &mut monitor),
            Status::Decode
        );
        assert!(monitor.is_null());

        let mut out = robustness();
        let values = [0.0f64];
        assert_eq!(
            sentil_embedded_update(ptr::null_mut(), 0.0, values.as_ptr(), 1, &mut out),
            Status::NullPointer
        );
        sentil_embedded_destroy(ptr::null_mut());
    }
}

#[cfg(feature = "synthesis")]
mod synth {
    use crate::formula::sentil_embedded_formula_create;
    use crate::synthesis::*;
    use crate::Status;
    use core::ptr;
    use sentil::{Bounds, Formula, LinearModel};
    use std::ffi::CString;

    #[test]
    fn numerics_and_filter() {
        unsafe {
            let spd = [2.0, 0.0, 0.0, 2.0];
            let rhs = [4.0, 6.0];
            let mut x = [0.0f64; 2];
            assert_eq!(
                sentil_embedded_solve_spd(spd.as_ptr(), 2, rhs.as_ptr(), x.as_mut_ptr()),
                Status::Ok
            );
            assert!((x[0] - 2.0).abs() < 1e-9 && (x[1] - 3.0).abs() < 1e-9);

            let lower = [-1.0, -1.0, -1.0];
            let upper = [1.0, 1.0, 1.0];
            let mut bounds: *mut Bounds = ptr::null_mut();
            assert_eq!(
                sentil_embedded_bounds_create(lower.as_ptr(), upper.as_ptr(), 3, &mut bounds),
                Status::Ok
            );
            let mut filter: *mut EmbeddedSafetyFilter = ptr::null_mut();
            assert_eq!(sentil_embedded_safety_filter_create(bounds, &mut filter), Status::Ok);
            let nominal = [2.0, 0.5, -3.0];
            let mut out = [0.0f64; 3];
            assert_eq!(
                sentil_embedded_safety_filter_filter(
                    filter,
                    nominal.as_ptr(),
                    3,
                    ptr::null(),
                    ptr::null(),
                    0,
                    out.as_mut_ptr()
                ),
                Status::Ok
            );
            assert!((out[0] - 1.0).abs() < 1e-9 && (out[2] + 1.0).abs() < 1e-9);

            assert_eq!(
                sentil_embedded_safety_filter_filter(
                    filter,
                    nominal.as_ptr(),
                    2,
                    ptr::null(),
                    ptr::null(),
                    0,
                    out.as_mut_ptr()
                ),
                Status::InvalidConfig
            );
            sentil_embedded_safety_filter_destroy(filter);
        }
    }

    #[test]
    fn synthesize_and_controller() {
        unsafe {
            let a = [1.0];
            let b = [1.0];
            let x0 = [1.0];
            let var = CString::new("x").unwrap();
            let vars = [var.as_ptr()];

            let mut model: *mut LinearModel = ptr::null_mut();
            assert_eq!(
                sentil_embedded_linear_model_create(
                    a.as_ptr(),
                    1,
                    b.as_ptr(),
                    1,
                    x0.as_ptr(),
                    vars.as_ptr(),
                    1.0,
                    2,
                    &mut model
                ),
                Status::Ok
            );
            let mut spec: *mut Formula = ptr::null_mut();
            let f = CString::new("x > 0").unwrap();
            assert_eq!(sentil_embedded_formula_create(f.as_ptr(), &mut spec), Status::Ok);

            let mut input = [0.0f64; 2];
            let mut robustness = 0.0;
            let mut holds = false;
            assert_eq!(
                sentil_embedded_synthesize(
                    model,
                    spec,
                    ptr::null(),
                    1,
                    3,
                    input.as_mut_ptr(),
                    &mut robustness,
                    &mut holds
                ),
                Status::Ok
            );

            let mut controller: *mut EmbeddedController = ptr::null_mut();
            assert_eq!(
                sentil_embedded_controller_create(model, spec, 1, 3, ptr::null(), &mut controller),
                Status::Ok
            );
            let state = [0.5f64];
            let mut step = [0.0f64; 1];
            assert_eq!(
                sentil_embedded_controller_control(controller, state.as_ptr(), 1, step.as_mut_ptr()),
                Status::Ok
            );
            sentil_embedded_controller_destroy(controller);
        }
    }
}