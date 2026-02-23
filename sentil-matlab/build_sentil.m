function build_sentil()
%BUILD_SENTIL Compile the SENTIL MEX gateway and Simulink S-Function.
%   Builds libsentil with cargo if it is missing, then compiles the
%   command-dispatch MEX and the Level-2 S-Function against the C ABI header and
%   copies the SENTIL library beside each one, so the packaged toolbox finds the
%   library next to the MEX with no build tree on the path and no environment
%   variables. On Linux and macOS a loader-relative rpath points at the copied
%   library; on Windows the loader searches the MEX's own folder, where the copied
%   sentil.dll sits.

base = fileparts(mfilename('fullpath'));
include = fullfile(base, '..', 'sentil-ffi', 'include');
libdir = fullfile(base, '..', 'target', 'release');
ext = sharedExt();

if ~exist(include, 'dir')
    error('sentil:build', 'C ABI headers not found at %s', include);
end
if ~exist(fullfile(libdir, ['libsentil.' ext]), 'file')
    fprintf('libsentil not built, running cargo...\n');
    root = fullfile(base, '..');
    status = system(sprintf('cargo build --release --package=sentil-ffi --manifest-path "%s"', ...
        fullfile(root, 'Cargo.toml')));
    if status ~= 0
        error('sentil:build', 'cargo build failed');
    end
end
% Resolve to an absolute path for the build-time link step.
here = cd(libdir);
libdir = pwd;
cd(here);
lib = fullfile(libdir, ['libsentil.' ext]);

mexDir = fullfile(base, '+sentil', 'private');
buildOne(include, libdir, lib, ext, fullfile(mexDir, 'sentil_mex.cpp'), mexDir);

sfun = fullfile(base, 'blocks', 'sentil_s_function.cpp');
if exist(sfun, 'file')
    buildOne(include, libdir, lib, ext, sfun, fullfile(base, 'blocks'));
end

addpath(base);
addpath(fullfile(base, 'blocks'));

fprintf('sentil-matlab built\n');
end

function buildOne(include, libdir, lib, ext, src, outdir)
% Compile one MEX and ship libsentil beside it. The rpath is relative to the loaded
% MEX ($ORIGIN on Linux, @loader_path on macOS), so the bundled library resolves
% wherever the toolbox installs. Windows needs no rpath: the loader searches the
% directory of the MEX, where the copied DLL sits.
flags = {['-I' include], ['-L' libdir], '-lsentil', '-R2018a'};
if ismac
    flags{end+1} = 'LDFLAGS=$LDFLAGS -Wl,-rpath,@loader_path';
elseif isunix
    flags{end+1} = 'LDFLAGS=$LDFLAGS -Wl,-rpath,\$ORIGIN';
end
fprintf('Building %s\n', src);
mex(flags{:}, src, '-outdir', outdir);
copyfile(lib, fullfile(outdir, ['libsentil.' ext]));
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