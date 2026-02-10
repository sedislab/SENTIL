module _Loader

import Libdl
using Artifacts
using LazyArtifacts

const _ENV_VAR = "SENTIL_LIB"

# cargo names the shared library `libsentil` on Unix but `sentil` on Windows.
_lib_names() = Sys.iswindows() ? ("sentil.dll", "libsentil.dll") : ("libsentil.$(Libdl.dlext)",)

function resolve()
    override = get(ENV, _ENV_VAR, "")
    if !isempty(override)
        isfile(override) || error("$_ENV_VAR is set to `$override`, but no file is there.")
        return override
    end
    return try
        from_artifact()
    catch err
        error(
            "libsentil was not found. Build the core with `cargo build --release -p " *
            "sentil-ffi` and export $_ENV_VAR=<repo>/target/release/$(first(_lib_names())), " *
            "or install Sentil from the registry, which ships the core. ($err)",
        )
    end
end

function from_artifact()
    dir = artifact"libsentil"
    for name in _lib_names()
        lib = joinpath(dir, name)
        isfile(lib) && return lib
    end
    error("the libsentil artifact does not contain $(join(_lib_names(), " or "))")
end

end