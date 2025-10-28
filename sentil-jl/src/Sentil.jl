module Sentil

include("loader.jl")

const libsentil = Ref{String}()

function __init__()
    libsentil[] = _Loader.resolve()
end

end