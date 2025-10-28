module _Loader

import Libdl

const _ENV_VAR = "SENTIL_LIB"

# Resolve the libsentil path once, at load. A developer who built the core locally
# exports SENTIL_LIB; a registry install carries the compiled core and sets the
# same variable from its artifact, so neither path needs a Rust toolchain at run.
function resolve()
    path = get(ENV, _ENV_VAR, "")
    if isempty(path)
        error(
            "libsentil was not found. Build the core with `cargo build --release -p " *
            "sentil-ffi` and export $_ENV_VAR=<repo>/target/release/libsentil.$(Libdl.dlext), " *
            "or install Sentil from the registry, which ships the core. See the README.",
        )
    end
    isfile(path) || error("$_ENV_VAR is set to `$path`, but no file is there.")
    return path
end

end