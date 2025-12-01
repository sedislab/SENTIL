% Offline robustness over a recorded trace, in discrete and dense time.

trace = sentil.Trace([0 1 2 3 4], 'speed', [12 9 7 4 6]);
phi = sentil.Formula.parse('always (speed > 5)');

fprintf('robustness: %g\n', phi.robustness(trace));
fprintf('per sample: %s\n', mat2str(phi.robustness_signal(trace)));

spans = phi.violations(trace);
fprintf('violations:');
for i = 1:size(spans, 1)
    fprintf(' [%g, %g]', spans(i, 1), spans(i, 2));
end
fprintf('\n');

fprintf('dense robustness: %g\n', phi.robustness_dense(trace));