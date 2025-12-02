function out = package_sentil(outputFile)
%PACKAGE_SENTIL Build the Sentil.mltbx toolbox package for the File Exchange.
arguments
    outputFile (1, 1) string = "Sentil.mltbx"
end

if ~exist('matlab.addons.toolbox.ToolboxOptions', 'class')
    error('sentil:packaging', ...
        'packaging needs MATLAB R2023a or newer for ToolboxOptions; got %s', version('-release'));
end

root = fileparts(mfilename('fullpath'));
v = sentil.version();
identifier = '3d7a1f6c-0b2e-4e9a-9c1d-5f8b2a4c6e10';

opts = matlab.addons.toolbox.ToolboxOptions(root, identifier);
opts.ToolboxName = 'Sentil';
opts.ToolboxVersion = sprintf('%d.%d.%d', v.major, v.minor, v.patch);
opts.Summary = 'Runtime verification for Signal Temporal Logic and its probabilistic extension.';
opts.Description = ['SENTIL monitors a signal against an STL or PrSTL formula, estimates ' ...
    'how likely a probabilistic specification holds, and synthesizes inputs and ' ...
    'controllers that satisfy a specification. The sentil package is the programmatic ' ...
    'API and the SENTIL Monitor block runs the streaming monitor inside Simulink.'];
opts.AuthorName = 'Paapa Kwesi Quansah';
opts.AuthorCompany = 'SEDIS lab, Baylor University';
opts.MinimumMatlabRelease = 'R2021b';
opts.OutputFile = outputFile;

opts.ToolboxFiles = setdiff(opts.ToolboxFiles, ...
    opts.ToolboxFiles(contains(opts.ToolboxFiles, ["slprj", ".git", "package_sentil"])));

matlab.addons.toolbox.packageToolbox(opts);
out = outputFile;
fprintf('packaged %s (version %s)\n', outputFile, opts.ToolboxVersion);
end