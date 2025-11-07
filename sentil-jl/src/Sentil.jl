module Sentil

import Base: minimum, maximum
import Statistics: mean, var, std

include("loader.jl")

const libsentil = Ref{String}()

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
include("simulation.jl")

function __init__()
    libsentil[] = _Loader.resolve()
    _C_BERNOULLI[] = @cfunction(_bernoulli_trampoline, Bool, (Ptr{Cvoid},))
end

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