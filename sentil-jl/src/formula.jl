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

export Formula, formula