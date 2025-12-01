% Probabilistic monitoring over a lifted noisy sensor.

times = 0:19;
values = 0.4 + 0.05 * times;
trace = sentil.Trace(times, 'x', values);

lifting = sentil.LiftingRegistry();
lifting.register('x', sentil.NoiseModel.gaussian(0.0, 0.3));

phi = sentil.Formula.parse('P>=0.9 (always (x > 0))');
config = sentil.SmcConfig;
config.samples = 5000;

result = phi.check(trace, lifting, config);
fprintf('probability %.3f, interval [%.3f, %.3f], holds %d\n', ...
    result.probability, result.interval.lower, result.interval.upper, result.holds);