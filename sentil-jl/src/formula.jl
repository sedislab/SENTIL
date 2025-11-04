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

export variable, literal

# Matching sentil_binary_op_t.
const _BIN_ADD = Int32(0)
const _BIN_SUB = Int32(1)
const _BIN_MUL = Int32(2)
const _BIN_DIV = Int32(3)
const _BIN_MOD = Int32(4)
const _BIN_POW = Int32(5)

function _binary(op::Int32, left::Expr, right::Expr)
    l, r = _consume_all!(left, right)
    Expr(ccall((:sentil_expr_binary, libsentil[]), Ptr{Cvoid},
               (Int32, Ptr{Cvoid}, Ptr{Cvoid}), op, l, r))
end

for (op, code) in ((:+, _BIN_ADD), (:-, _BIN_SUB), (:*, _BIN_MUL), (:/, _BIN_DIV))
    @eval begin
        Base.$op(a::Expr, b::Expr) = _binary($code, a, b)
        Base.$op(a::Expr, b::Real) = _binary($code, a, literal(b))
        Base.$op(a::Real, b::Expr) = _binary($code, literal(a), b)
    end
end

Base.:-(e::Expr) = _binary(_BIN_SUB, literal(0.0), e)

Base.mod(a::Expr, b::Expr) = _binary(_BIN_MOD, a, b)
Base.mod(a::Expr, b::Real) = _binary(_BIN_MOD, a, literal(b))

"""Raise one term to another."""
pow(a::Expr, b::Expr) = _binary(_BIN_POW, a, b)
pow(a::Expr, b::Real) = _binary(_BIN_POW, a, literal(b))
Base.:^(a::Expr, b::Expr) = pow(a, b)
Base.:^(a::Expr, b::Real) = pow(a, literal(b))
Base.literal_pow(::typeof(^), a::Expr, ::Val{p}) where {p} = pow(a, literal(p))

export pow

function _call(name::AbstractString, args::Expr...)
    ptrs = collect(Ptr{Cvoid}, _consume_all!(args...))
    Expr(ccall((:sentil_expr_call, libsentil[]), Ptr{Cvoid},
               (Cstring, Ptr{Ptr{Cvoid}}, Csize_t), name, ptrs, length(ptrs)))
end

for fn in (:abs, :sin, :cos, :tan, :sqrt, :exp, :log, :floor, :ceil)
    @eval Base.$fn(e::Expr) = _call($(string(fn)), e)
end

"""The natural logarithm of a term."""
ln(e::Expr) = _call("ln", e)

Base.min(a::Expr, b::Expr) = _call("min", a, b)
Base.min(a::Expr, b::Real) = _call("min", a, literal(b))
Base.min(a::Real, b::Expr) = _call("min", literal(a), b)
Base.max(a::Expr, b::Expr) = _call("max", a, b)
Base.max(a::Expr, b::Real) = _call("max", a, literal(b))
Base.max(a::Real, b::Expr) = _call("max", literal(a), b)

export ln

# Matching sentil_comparison_op_t.
const _CMP_LT = Int32(0)
const _CMP_LE = Int32(1)
const _CMP_GT = Int32(2)
const _CMP_GE = Int32(3)
const _CMP_EQ = Int32(4)
const _CMP_NE = Int32(5)

function _predicate(lhs::Expr, op::Int32, rhs::Expr)
    l, r = _consume_all!(lhs, rhs)
    Formula(ccall((:sentil_formula_predicate, libsentil[]), Ptr{Cvoid},
                  (Ptr{Cvoid}, Int32, Ptr{Cvoid}), l, op, r))
end

for (op, code) in ((:<, _CMP_LT), (:(<=), _CMP_LE), (:>, _CMP_GT),
                   (:(>=), _CMP_GE), (:(==), _CMP_EQ), (:(!=), _CMP_NE))
    @eval begin
        Base.$op(a::Expr, b::Expr) = _predicate(a, $code, b)
        Base.$op(a::Expr, b::Real) = _predicate(a, $code, literal(b))
        Base.$op(a::Real, b::Expr) = _predicate(literal(a), $code, b)
    end
end

Base.:!(f::Formula) =
    Formula(ccall((:sentil_formula_not, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _consume!(f)))
Base.:~(f::Formula) = !f

function Base.:&(a::Formula, b::Formula)
    l, r = _consume_all!(a, b)
    Formula(ccall((:sentil_formula_and, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid}, Ptr{Cvoid}), l, r))
end

function Base.:|(a::Formula, b::Formula)
    l, r = _consume_all!(a, b)
    Formula(ccall((:sentil_formula_or, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid}, Ptr{Cvoid}), l, r))
end

and(a::Formula, b::Formula) = a & b
or(a::Formula, b::Formula) = a | b

"""
    implies(a, b) -> Formula

The formula `a → b`, equal to `!a | b`.
"""
function implies(a::Formula, b::Formula)
    l, r = _consume_all!(a, b)
    Formula(ccall((:sentil_formula_implies, libsentil[]), Ptr{Cvoid},
                  (Ptr{Cvoid}, Ptr{Cvoid}), l, r))
end

"""The formula holds at the next step."""
next(f::Formula) =
    Formula(ccall((:sentil_formula_next, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _consume!(f)))

export and, or, implies, next