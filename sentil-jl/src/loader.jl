module _Loader

import Libdl
using Artifacts
using LazyArtifacts

const _ENV_VAR = "SENTIL_LIB"

# Resolve libsentil once, in __init__ rather than at precompile. A developer who
# built the core locally points SENTIL_LIB at it; a registry install has no such
# variable and pulls the platform's prebuilt core from the package artifact. Neither
# path needs a Rust toolchain at run time.
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
            "sentil-ffi` and export $_ENV_VAR=<repo>/target/release/libsentil.$(Libdl.dlext), " *
            "or install Sentil from the registry, which ships the core. ($err)",
        )
    end
end

# The library inside the platform artifact the release attaches, downloaded lazily on
# first use. The tarball places libsentil at the artifact root.
function from_artifact()
    dir = artifact"libsentil"
    lib = joinpath(dir, "libsentil.$(Libdl.dlext)")
    isfile(lib) || error("the libsentil artifact does not contain $(basename(lib))")
    return lib
end

end