function v = version()
%SENTIL.VERSION The version of the linked SENTIL engine, as a struct.
raw = sentil_mex('version');
v = struct('major', raw(1), 'minor', raw(2), 'patch', raw(3));
end