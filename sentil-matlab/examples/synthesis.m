% Synthesize a control input that satisfies a spec, then shield it online.

model = sentil.SystemModel.linear(1, 1, 1, {'x'}, 1.0, 3);
spec = sentil.Formula.parse('always (x > 0)');
bounds = sentil.Bounds([-1 -1 -1], [1 1 1]);

result = sentil.Synthesis.synthesize(model, spec, bounds);
fprintf('input: %s robustness: %g holds: %d\n', mat2str(result.input), result.robustness, ...
    result.holds);

shield = sentil.SafetyFilter(sentil.Bounds([-1 -1 -1], [1 1 1]));
fprintf('shielded: %s\n', mat2str(shield.filter([2.0 0.5 -3.0])));