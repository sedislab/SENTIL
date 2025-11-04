# A parsed PrSTL formula. It owns a handle into the core's formula tree; the finalizer
# frees it, or call close! to free it eagerly.

"""
    Formula

A signal temporal logic formula, possibly probabilistic. Build one by parsing text
with `parse(Formula, text)` or `formula(text)`, by composing predicates and operators,
or from JSON with `from_json(Formula, json)`.
"""
mutable struct Formula
    ptr::Ptr{Cvoid}
    function Formula(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        f = new(ptr)
        finalizer(_destroy, f)
        return f
    end
end

function _destroy(f::Formula)
    if f.ptr != C_NULL
        ccall((:sentil_formula_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), f.ptr)
        f.ptr = C_NULL
    end
end

close!(f::Formula) = _destroy(f)

"""Parse a PrSTL formula from its textual form."""
Base.parse(::Type{Formula}, text::AbstractString) =
    Formula(ccall((:sentil_formula_parse, libsentil[]), Ptr{Cvoid}, (Cstring,), text))

formula(text::AbstractString) = parse(Formula, text)

"""The formula as JSON, the form `from_json` reads back."""
to_json(f::Formula) =
    _take_string(ccall((:sentil_formula_to_json, libsentil[]), Ptr{UInt8}, (Ptr{Cvoid},), _ptr(f)))

"""Rebuild a formula from the output of `to_json`."""
from_json(::Type{Formula}, json::AbstractString) =
    Formula(ccall((:sentil_formula_from_json, libsentil[]), Ptr{Cvoid}, (Cstring,), json))

"""The nesting depth, where a predicate is 1."""
depth(f::Formula) =
    Int(ccall((:sentil_formula_depth, libsentil[]), Csize_t, (Ptr{Cvoid},), _ptr(f)))

"""Whether the formula carries a temporal operator."""
is_temporal(f::Formula) =
    ccall((:sentil_formula_has_temporal, libsentil[]), Bool, (Ptr{Cvoid},), _ptr(f))

"""The variable names the formula references, sorted and unique."""
function variables(f::Formula)
    count = Ref{Csize_t}(0)
    ptr = ccall((:sentil_formula_variables, libsentil[]), Ptr{Ptr{UInt8}},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(f), count)
    return _take_string_array(ptr, count[])
end

export Formula, formula, to_json, from_json, depth, is_temporal, variables

# A real-valued term over the signal variables. Compose terms with arithmetic and the
# math functions, then compare two terms to get a predicate Formula. The arithmetic and
# call builders consume their operands, so a term is single use once combined.
mutable struct Expr
    ptr::Ptr{Cvoid}
    function Expr(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        e = new(ptr)
        finalizer(_destroy, e)
        return e
    end
end

function _destroy(e::Expr)
    if e.ptr != C_NULL
        ccall((:sentil_expr_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), e.ptr)
        e.ptr = C_NULL
    end
end

close!(e::Expr) = _destroy(e)

"""A term that reads the signal named `name`."""
variable(name::AbstractString) =
    Expr(ccall((:sentil_expr_variable, libsentil[]), Ptr{Cvoid}, (Cstring,), name))

"""A constant term."""
literal(value::Real) =
    Expr(ccall((:sentil_expr_literal, libsentil[]), Ptr{Cvoid}, (Cdouble,), value))

export Expr, variable, literal