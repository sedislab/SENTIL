use crate::conversions::{
    c_char_to_string, clear_error, ffi_panic_boundary, into_string_array, set_error, to_c_string,
};
use crate::handles::{drop_handle, into_handle, take_handle};
use crate::{SentilBinaryOp, SentilComparisonOp, SentilError, SentilProbabilityOp};
use libc::{c_char, c_double, c_void, size_t};
use sentil::formula::{Expr, Interval, Predicate};
use sentil::Formula;
use std::ptr;

fn unary_formula(child: *mut c_void, build: fn(Box<Formula>) -> Formula) -> *mut c_void {
    let Some(inner) = (unsafe { take_handle::<Formula>(child) }) else {
        set_error(SentilError::NullPointer, "the child formula was null");
        return ptr::null_mut();
    };
    into_handle(build(Box::new(inner)))
}

pub(crate) fn aliased(left: *mut c_void, right: *mut c_void) -> bool {
    if !left.is_null() && left == right {
        set_error(
            SentilError::InvalidConfig,
            "the same handle was passed as both operands, and each one is consumed; \
             build the operand twice so there are two handles",
        );
        return true;
    }
    false
}

pub(crate) unsafe fn repeated_arg(args: *mut *mut c_void, count: usize) -> bool {
    for i in 0..count {
        let a = unsafe { *args.add(i) };
        if a.is_null() {
            continue;
        }
        for j in (i + 1)..count {
            if a == unsafe { *args.add(j) } {
                set_error(
                    SentilError::InvalidConfig,
                    "the same handle appears twice in the argument list, and each one is \
                     consumed; build the argument once per position",
                );
                return true;
            }
        }
    }
    false
}

fn binary_formula(
    left: *mut c_void,
    right: *mut c_void,
    build: fn(Box<Formula>, Box<Formula>) -> Formula,
) -> *mut c_void {
    if aliased(left, right) {
        return ptr::null_mut();
    }
    let (Some(l), Some(r)) =
        (unsafe { take_handle::<Formula>(left) }, unsafe { take_handle::<Formula>(right) })
    else {
        set_error(SentilError::NullPointer, "a child formula was null");
        return ptr::null_mut();
    };
    into_handle(build(Box::new(l), Box::new(r)))
}

fn interval_from(lower: f64, upper: f64, has_upper: bool) -> Option<Interval> {
    match Interval::new(lower, has_upper.then_some(upper)) {
        Ok(i) => Some(i),
        Err(e) => {
            let _: SentilError = e.into();
            None
        }
    }
}

fn interval_formula(
    lower: f64,
    upper: f64,
    has_upper: bool,
    child: *mut c_void,
    build: fn(Interval, Box<Formula>) -> Formula,
) -> *mut c_void {
    let Some(inner) = (unsafe { take_handle::<Formula>(child) }) else {
        set_error(SentilError::NullPointer, "the child formula was null");
        return ptr::null_mut();
    };
    let Some(interval) = interval_from(lower, upper, has_upper) else {
        return ptr::null_mut();
    };
    into_handle(build(interval, Box::new(inner)))
}

fn interval_binary_formula(
    lower: f64,
    upper: f64,
    has_upper: bool,
    left: *mut c_void,
    right: *mut c_void,
    build: fn(Interval, Box<Formula>, Box<Formula>) -> Formula,
) -> *mut c_void {
    if aliased(left, right) {
        return ptr::null_mut();
    }
    let (Some(l), Some(r)) =
        (unsafe { take_handle::<Formula>(left) }, unsafe { take_handle::<Formula>(right) })
    else {
        set_error(SentilError::NullPointer, "a child formula was null");
        return ptr::null_mut();
    };
    let Some(interval) = interval_from(lower, upper, has_upper) else {
        return ptr::null_mut();
    };
    into_handle(build(interval, Box::new(l), Box::new(r)))
}

#[no_mangle]
pub extern "C" fn sentil_formula_parse(input: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(input) else {
            return ptr::null_mut();
        };
        match Formula::parse(&text) {
            Ok(formula) => into_handle(formula),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Formula>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_formula_to_json(handle: *mut c_void) -> *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let formula = borrow_handle!(handle, Formula, ptr::null_mut());
        match serde_json::to_string(formula) {
            Ok(text) => to_c_string(&text),
            Err(e) => {
                set_error(SentilError::Json, &e.to_string());
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_from_json(json: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(json) else {
            return ptr::null_mut();
        };
        match serde_json::from_str::<Formula>(&text) {
            Ok(formula) => into_handle(formula),
            Err(e) => {
                set_error(SentilError::Json, &e.to_string());
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_depth(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || {
        let formula = borrow_handle!(handle, Formula, 0);
        formula.depth()
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_has_temporal(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(false, || {
        let formula = borrow_handle!(handle, Formula, false);
        formula.has_temporal()
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_variables(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let formula = borrow_handle!(handle, Formula, ptr::null_mut());
        into_string_array(formula.variables(), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_expr_variable(name: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        into_handle(Expr::Variable(name))
    })
}

#[no_mangle]
pub extern "C" fn sentil_expr_literal(value: c_double) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(Expr::Literal(value)))
}

#[no_mangle]
pub extern "C" fn sentil_expr_binary(
    op: SentilBinaryOp,
    left: *mut c_void,
    right: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        if aliased(left, right) {
            return ptr::null_mut();
        }
        let (Some(l), Some(r)) =
            (unsafe { take_handle::<Expr>(left) }, unsafe { take_handle::<Expr>(right) })
        else {
            set_error(SentilError::NullPointer, "a child expression was null");
            return ptr::null_mut();
        };
        into_handle(Expr::Binary(op.into(), Box::new(l), Box::new(r)))
    })
}

#[no_mangle]
pub extern "C" fn sentil_expr_call(
    name: *const c_char,
    args: *mut *mut c_void,
    count: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        if count > 0 {
            check_ptr!(args, ptr::null_mut());
        }
        if unsafe { repeated_arg(args, count) } {
            return ptr::null_mut();
        }
        let mut taken: Vec<Option<Expr>> = Vec::with_capacity(count);
        for i in 0..count {
            taken.push(unsafe { take_handle::<Expr>(*args.add(i)) });
        }
        if taken.iter().any(Option::is_none) {
            set_error(SentilError::NullPointer, "an argument expression was null");
            return ptr::null_mut();
        }
        into_handle(Expr::Call(name, taken.into_iter().flatten().collect()))
    })
}

#[no_mangle]
pub extern "C" fn sentil_expr_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Expr>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_formula_predicate(
    lhs: *mut c_void,
    op: SentilComparisonOp,
    rhs: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        if aliased(lhs, rhs) {
            return ptr::null_mut();
        }
        let (Some(l), Some(r)) =
            (unsafe { take_handle::<Expr>(lhs) }, unsafe { take_handle::<Expr>(rhs) })
        else {
            set_error(SentilError::NullPointer, "a predicate operand was null");
            return ptr::null_mut();
        };
        into_handle(Formula::Predicate(Predicate { lhs: l, op: op.into(), rhs: r }))
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_not(child: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || unary_formula(child, Formula::Not))
}

#[no_mangle]
pub extern "C" fn sentil_formula_and(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || binary_formula(left, right, Formula::And))
}

#[no_mangle]
pub extern "C" fn sentil_formula_or(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || binary_formula(left, right, Formula::Or))
}

#[no_mangle]
pub extern "C" fn sentil_formula_implies(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || binary_formula(left, right, Formula::Implies))
}

#[no_mangle]
pub extern "C" fn sentil_formula_next(child: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || unary_formula(child, Formula::Next))
}

#[no_mangle]
pub extern "C" fn sentil_formula_always(
    lower: c_double,
    upper: c_double,
    has_upper: bool,
    child: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        interval_formula(lower, upper, has_upper, child, Formula::Always)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_eventually(
    lower: c_double,
    upper: c_double,
    has_upper: bool,
    child: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        interval_formula(lower, upper, has_upper, child, Formula::Eventually)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_historically(
    lower: c_double,
    upper: c_double,
    has_upper: bool,
    child: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        interval_formula(lower, upper, has_upper, child, Formula::Historically)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_once(
    lower: c_double,
    upper: c_double,
    has_upper: bool,
    child: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        interval_formula(lower, upper, has_upper, child, Formula::Once)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_until(
    lower: c_double,
    upper: c_double,
    has_upper: bool,
    left: *mut c_void,
    right: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        interval_binary_formula(lower, upper, has_upper, left, right, Formula::Until)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_since(
    lower: c_double,
    upper: c_double,
    has_upper: bool,
    left: *mut c_void,
    right: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        interval_binary_formula(lower, upper, has_upper, left, right, Formula::Since)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_probabilistic(
    op: SentilProbabilityOp,
    threshold: c_double,
    child: *mut c_void,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Some(inner) = (unsafe { take_handle::<Formula>(child) }) else {
            set_error(SentilError::NullPointer, "the child formula was null");
            return ptr::null_mut();
        };
        if !(0.0..=1.0).contains(&threshold) {
            set_error(
                SentilError::InvalidConfig,
                &format!("probability threshold {threshold} is outside [0, 1]"),
            );
            return ptr::null_mut();
        }
        into_handle(Formula::Probabilistic(op.into(), threshold, Box::new(inner)))
    })
}