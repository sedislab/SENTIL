module Sentil

include("loader.jl")

const libsentil = Ref{String}()

function __init__()
    libsentil[] = _Loader.resolve()
end

include("errors.jl")
include("enums.jl")
include("value_structs.jl")

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