function build_sentil()
%BUILD_SENTIL Compile the SENTIL MEX gateway and Simulink S-Function.

base = fileparts(mfilename('fullpath'));
include = fullfile(base, '..', 'sentil-ffi', 'include');
libdir = fullfile(base, '..', 'target', 'release');
runtime = runtimeLib();

if ~exist(include, 'dir')
    error('sentil:build', 'C ABI headers not found at %s', include);
end
if ~exist(fullfile(libdir, runtime), 'file')
    fprintf('libsentil not built, running cargo...\n');
    root = fullfile(base, '..');
    status = system(sprintf('cargo build --release --package=sentil-ffi --manifest-path "%s"', ...
        fullfile(root, 'Cargo.toml')));
    if status ~= 0
        error('sentil:build', 'cargo build failed');
    end
end
here = cd(libdir);
libdir = pwd;
cd(here);
lib = fullfile(libdir, runtime);

mexDir = fullfile(base, '+sentil', 'private');
buildOne(include, libdir, lib, runtime, fullfile(mexDir, 'sentil_mex.cpp'), mexDir);

sfun = fullfile(base, 'blocks', 'sentil_s_function.cpp');
if exist(sfun, 'file')
    buildOne(include, libdir, lib, runtime, sfun, fullfile(base, 'blocks'));
end

addpath(base);
addpath(fullfile(base, 'blocks'));

fprintf('sentil-matlab built\n');
end

function buildOne(include, libdir, lib, runtime, src, outdir)
% $ORIGIN and @loader_path resolve the bundled library relative to the loaded MEX.
flags = {['-I' include], '-R2018a'};
if ispc
    % MSVC resolves -lsentil to the static sentil.lib, not the DLL's import library.
    flags{end + 1} = ['LINKLIBS=$LINKLIBS "' importLib(libdir) '"'];
elseif ismac
    flags = [flags, {['-L' libdir], '-lsentil', 'LDFLAGS=$LDFLAGS -Wl,-rpath,@loader_path'}];
else
    flags = [flags, {['-L' libdir], '-lsentil', 'LDFLAGS=$LDFLAGS -Wl,-rpath,\$ORIGIN'}];
end
fprintf('Building %s\n', src);
mex(flags{:}, src, '-outdir', outdir);
copyfile(lib, fullfile(outdir, runtime));
end

function name = runtimeLib()
if ispc
    name = 'sentil.dll';
elseif ismac
    name = 'libsentil.dylib';
else
    name = 'libsentil.so';
end
end

function path = importLib(libdir)
% Newer cargo leaves the import library in target/release, older cargo only in deps.
candidates = {fullfile(libdir, 'sentil.dll.lib'), fullfile(libdir, 'deps', 'sentil.dll.lib')};
for k = 1:numel(candidates)
    if exist(candidates{k}, 'file')
        path = candidates{k};
        return
    end
end
error('sentil:build', 'import library sentil.dll.lib not found under %s', libdir);
end