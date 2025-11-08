mutable struct SpecBuilder
    ptr::Ptr{Cvoid}
    function SpecBuilder(ptr::Ptr{Cvoid})
        ptr == C_NULL && _raise_last()
        b = new(ptr)
        finalizer(_destroy, b)
        return b
    end
end

function _destroy(b::SpecBuilder)
    if b.ptr != C_NULL
        ccall((:sentil_spec_builder_destroy, libsentil[]), Cvoid, (Ptr{Cvoid},), b.ptr)
        b.ptr = C_NULL
    end
end

close!(b::SpecBuilder) = _destroy(b)

"""The names of the specifications in the library, such as `aerospace/altitude_hold`."""
function available_specs()
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_spec_registry_available, libsentil[]), Ptr{Ptr{UInt8}}, (Ptr{Csize_t},), n)
    return _take_string_array(ptr, n[])
end

"""A builder for a library specification by name, or one read from a spec file."""
SpecBuilder(name::AbstractString) =
    SpecBuilder(ccall((:sentil_spec_builder_create, libsentil[]), Ptr{Cvoid}, (Cstring,), name))

SpecBuilder(; file::AbstractString) =
    SpecBuilder(ccall((:sentil_spec_builder_from_file, libsentil[]), Ptr{Cvoid}, (Cstring,), file))

"""Select a named variant, consuming the input builder."""
with_variant(b::SpecBuilder, variant::AbstractString) =
    SpecBuilder(ccall((:sentil_spec_builder_with_variant, libsentil[]), Ptr{Cvoid},
                      (Ptr{Cvoid}, Cstring), _consume!(b), variant))

"""Set a parameter, consuming the input builder."""
with_param(b::SpecBuilder, name::AbstractString, value::Real) =
    SpecBuilder(ccall((:sentil_spec_builder_with_param, libsentil[]), Ptr{Cvoid},
                      (Ptr{Cvoid}, Cstring, Cdouble), _consume!(b), name, value))

"""The variants the specification offers."""
function available_variants(b::SpecBuilder)
    n = Ref{Csize_t}(0)
    ptr = ccall((:sentil_spec_builder_available_variants, libsentil[]), Ptr{Ptr{UInt8}},
                (Ptr{Cvoid}, Ptr{Csize_t}), _ptr(b), n)
    return _take_string_array(ptr, n[])
end

"""The deterministic specification as PrSTL text."""
build_deterministic(b::SpecBuilder) =
    _take_string(ccall((:sentil_spec_builder_build_deterministic, libsentil[]), Ptr{UInt8}, (Ptr{Cvoid},), _ptr(b)))

"""The probabilistic specification as PrSTL text."""
build_probabilistic(b::SpecBuilder) =
    _take_string(ccall((:sentil_spec_builder_build_probabilistic, libsentil[]), Ptr{UInt8}, (Ptr{Cvoid},), _ptr(b)))

"""The deterministic specification as a `Formula`."""
build_formula(b::SpecBuilder) =
    Formula(ccall((:sentil_spec_builder_build_formula, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _ptr(b)))

"""The probabilistic specification as a `Formula`."""
build_probabilistic_formula(b::SpecBuilder) =
    Formula(ccall((:sentil_spec_builder_build_probabilistic_formula, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _ptr(b)))

"""A lifting registry carrying the specification's noise models."""
build_lifting_registry(b::SpecBuilder) =
    LiftingRegistry(ccall((:sentil_spec_builder_build_lifting_registry, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _ptr(b)))

"""The resolved parameters as a JSON object."""
parameters_json(b::SpecBuilder) =
    _take_string(ccall((:sentil_spec_builder_parameters_json, libsentil[]), Ptr{UInt8}, (Ptr{Cvoid},), _ptr(b)))

"""A monitor for the specification, consuming the builder."""
build_monitor(b::SpecBuilder) =
    Monitor(ccall((:sentil_spec_builder_into_monitor, libsentil[]), Ptr{Cvoid}, (Ptr{Cvoid},), _consume!(b)))

"""The SMC settings the specification recommends, or `nothing`."""
function smc_settings(b::SpecBuilder)
    out = Ref{SpecSmcSettings}()
    has = ccall((:sentil_spec_builder_smc_settings, libsentil[]), Bool, (Ptr{Cvoid}, Ptr{SpecSmcSettings}), _ptr(b), out)
    return has ? out[] : nothing
end

"""The SPRT settings the specification recommends, or `nothing`."""
function sprt_settings(b::SpecBuilder)
    out = Ref{SpecSprtSettings}()
    has = ccall((:sentil_spec_builder_sprt_settings, libsentil[]), Bool, (Ptr{Cvoid}, Ptr{SpecSprtSettings}), _ptr(b), out)
    return has ? out[] : nothing
end

"""The rare-event settings the specification recommends, or `nothing`."""
function ams_settings(b::SpecBuilder)
    out = Ref{SpecAmsSettings}()
    has = ccall((:sentil_spec_builder_ams_settings, libsentil[]), Bool, (Ptr{Cvoid}, Ptr{SpecAmsSettings}), _ptr(b), out)
    return has ? out[] : nothing
end

export SpecBuilder, available_specs, with_variant, with_param, available_variants
export build_deterministic, build_probabilistic, build_formula, build_probabilistic_formula
export build_lifting_registry, parameters_json, build_monitor, smc_settings, sprt_settings, ams_settings