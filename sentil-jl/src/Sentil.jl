module Sentil

import Base: minimum, maximum
import Statistics: mean, var, std

include("loader.jl")

const libsentil = Ref{String}()

function __init__()
    libsentil[] = _Loader.resolve()
    # Host-callback trampolines must be built at run time, not baked into the
    # precompiled image, or the engine would call through a stale pointer.
    _C_BERNOULLI[] = @cfunction(_bernoulli_trampoline, Bool, (Ptr{Cvoid},))
end

include("errors.jl")
include("enums.jl")
include("value_structs.jl")
include("handles.jl")
include("trace.jl")
include("formula.jl")
include("monitor.jl")
include("noise.jl")
include("stats.jl")
include("sequential.jl")

"""The version of the SENTIL core as `(major, minor, patch)`."""
function version()
    major = Ref{UInt32}(0)
    minor = Ref{UInt32}(0)
    patch = Ref{UInt32}(0)
    ccall((:sentil_version, libsentil[]), Cvoid,
          (Ptr{UInt32}, Ptr{UInt32}, Ptr{UInt32}), major, minor, patch)
    return (Int(major[]), Int(minor[]), Int(patch[]))
end

export version

end