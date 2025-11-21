function build_sentil()
%BUILD_SENTIL Compile the SENTIL MEX gateway and Simulink S-Function.
%   Builds libsentil with cargo if it is missing, then compiles the
%   command-dispatch MEX and the Level-2 S-Function against the C ABI header,
%   linking libsentil with its directory baked in as an rpath so the compiled
%   .mexa64 finds the shared library without LD_LIBRARY_PATH.

base = fileparts(mfilename('fullpath'));
include = fullfile(base, '..', 'sentil-ffi', 'include');
libdir = fullfile(base, '..', 'target', 'release');

if ~exist(include, 'dir')
    error('sentil:build', 'C ABI headers not found at %s', include);
end
lib = fullfile(libdir, ['libsentil.' sharedExt()]);
if ~exist(lib, 'file')
    fprintf('libsentil not built, running cargo...\n');
    root = fullfile(base, '..');
    status = system(sprintf('cargo build --release --package=sentil-ffi --manifest-path "%s"', ...
        fullfile(root, 'Cargo.toml')));
    if status ~= 0
        error('sentil:build', 'cargo build failed');
    end
end
% Resolve to an absolute path for the rpath.
here = cd(libdir);
libdir = pwd;
cd(here);

flags = {['-I' include], ['-L' libdir], '-lsentil', '-R2018a', ...
    ['LDFLAGS=$LDFLAGS -Wl,-rpath,' libdir]};

mexFile = fullfile(base, '+sentil', 'private', 'sentil_mex.cpp');
fprintf('Building %s\n', mexFile);
mex(flags{:}, mexFile, '-outdir', fullfile(base, '+sentil', 'private'));

sfun = fullfile(base, 'blocks', 'sentil_s_function.cpp');
if exist(sfun, 'file')
    fprintf('Building %s\n', sfun);
    mex(flags{:}, sfun, '-outdir', fullfile(base, 'blocks'));
end

fprintf('sentil-matlab built\n');
end

function ext = sharedExt()
if ismac
    ext = 'dylib';
elseif ispc
    ext = 'dll';
else
    ext = 'so';
end
end