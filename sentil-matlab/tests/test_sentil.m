function tests = test_sentil
%TEST_SENTIL Per-capability tests for the SENTIL MATLAB binding.
tests = functiontests(localfunctions);
end

function testFormula(t)
phi = sentil.Formula.parse('always[0, 10](x > 0 and y < 5)');
verifyEqual(t, sort(phi.variables()), {'x', 'y'});
trace = sentil.Trace([0 1 2 3], 'x', [1 2 -1 3]);
g = sentil.Formula.parse('always[0,2](x > 0)');
verifyEqual(t, g.robustness(trace), -1);
verifyEqual(t, g.violations(trace), [0 2]);
built = sentil.Expr.var('x').mul(sentil.Expr.literal(2));
bf = sentil.Formula.predicate(built, sentil.ComparisonOp.Gt, sentil.Expr.literal(5));
pf = sentil.Formula.parse('x * 2 > 5');
verifyEqual(t, bf.robustness_signal(trace), pf.robustness_signal(trace));
end

function testTraceAndRingBuffer(t)
trace = sentil.Trace([0 2 4], 'x', [0 2 4]);
verifyEqual(t, trace.length(), 3);
r = trace.resample([0 1 2 3 4], sentil.Interpolation.Linear);
verifyEqual(t, r.signal('x'), [0 1 2 3 4]);
verifyEmpty(t, trace.signal('missing'));
b = sentil.RingBuffer(3);
b.push(0, 10); b.push(1, 20); b.push(2, 30);
evicted = b.push(3, 40);
verifyEqual(t, evicted.value, 10);
verifyEqual(t, b.front().value, 20);
s = sentil.RingBuffer(5);
s.push(0, 2); s.push(1, 4); s.push(2, 6);
verifyEqual(t, s.mean(), 4);
verifyEqual(t, s.variance(), 4);
verifyEmpty(t, sentil.RingBuffer(2).mean());
end

function testMonitors(t)
m = sentil.Monitor('always[0, 2](x > 0)');
trace = sentil.Trace([0 1 2 3], 'x', [1 2 -1 3]);
verifyEqual(t, m.robustness(trace), -1);
verifyEqual(t, m.symbol_index('x'), 1);
verifyEmpty(t, m.symbol_index('y'));
om = sentil.OnlineMonitor('always[0, 10](x > -0.9)');
violatedAt = -1;
for tt = 0:59
    v = om.update(tt, struct('x', sin(tt * 0.3)));
    if v.resolved && ~v.satisfied
        violatedAt = tt;
        break
    end
end
verifyEqual(t, violatedAt, 15);
mm = sentil.MultiMonitor();
mm.add('safety', 'x > 0');
r = mm.update(0, struct('x', 5));
verifyEqual(t, r('safety').value, 5);
b = sentil.FormulaBank();
b.add('p1', 'always(x > 0)');
rb = b.robustness(sentil.Trace([0 1 2], 'x', [3 -1 4]));
verifyEqual(t, rb('p1'), -1);
end

function testStats(t)
w = sentil.Stats.wilson(50, 100, 0.95);
verifyEqual(t, w.lower, 0.403832, 'AbsTol', 1e-6);
verifyEqual(t, w.upper, 0.596168, 'AbsTol', 1e-6);
verifyEqual(t, sentil.Stats.z_score(0.95), 1.95996, 'AbsTol', 1e-5);
verifyEqual(t, sentil.Stats.chernoff_hoeffding_samples(0.1, 0.05), 185);
end

function testNoiseAndLifting(t)
g = sentil.NoiseModel.gaussian(2, 3);
verifyEqual(t, g.mean(), 2);
verifyEqual(t, g.variance(), 9);
verifyEmpty(t, sentil.NoiseModel.cauchy(0, 1).mean());
res = sentil.NoiseModel.residuals([1 2 3 4], [1.1 2.0 3.2 3.9], sentil.NoiseInteraction.Additive);
verifyEqual(t, res, [0.1 0 0.2 -0.1], 'AbsTol', 1e-9);
mx = sentil.NoiseModel.mixture([0.5 0.5], ...
    [sentil.NoiseModel.gaussian(0, 1), sentil.NoiseModel.gaussian(10, 1)]);
verifyEqual(t, mx.mean(), 5, 'AbsTol', 1e-9);
reg = sentil.LiftingRegistry();
reg.register('x', sentil.NoiseModel.gaussian(0, 0.5));
trace = sentil.Trace([0 1 2], 'x', [5 5 5]);
a = reg.lift(trace, 42);
b = reg.lift(trace, 42);
verifyEqual(t, a.signal('x'), b.signal('x'));
end

function testStatisticalChecks(t)
trace = sentil.Trace(0:4, 'x', [3 3 3 3 3]);
reg = sentil.LiftingRegistry();
reg.register('x', sentil.NoiseModel.gaussian(0, 1));
phi = sentil.Formula.parse('P>=0.5(eventually[0,4](x > 2))');
r = phi.check(trace, reg);
verifyTrue(t, r.holds);
verifyGreaterThan(t, r.probability, 0.99);
sprt = phi.check_sequential(trace, reg, sentil.SprtConfig(0.4, 0.6));
verifyEqual(t, sprt.verdict, sentil.SprtVerdict.AcceptH1);
bayes = phi.check_bayesian(trace, reg, sentil.BayesConfig(0.5));
verifyEqual(t, bayes.verdict, sentil.BayesVerdict.Holds);
end

function testSynthesis(t)
m = sentil.SystemModel.linear(1, 1, 1, {'x'}, 1.0, 3);
r = sentil.Synthesis.synthesize(m, sentil.Formula.parse('always (x > 0)'), ...
    sentil.Bounds([-1 -1 -1], [1 1 1]));
verifyEqual(t, r.input, [1 -1 0], 'AbsTol', 1e-6);
verifyTrue(t, r.holds);
sf = sentil.SafetyFilter(sentil.Bounds([-1 -1 -1], [1 1 1]));
verifyEqual(t, sf.filter([2 0.5 -3]), [1 0.5 -1], 'AbsTol', 1e-9);
verifyEqual(t, sentil.Numerics.solve_spd([2 0; 0 2], [4 6]), [2 3], 'AbsTol', 1e-9);
e = sentil.Numerics.symmetric_eigen([2 0; 0 3]);
verifyEqual(t, sort(e.values), [2 3], 'AbsTol', 1e-9);
end

function testWitnessesAndOptimizers(t)
m = sentil.SystemModel.linear(1, 1, 0, {'x'}, 1.0, 4);
w = sentil.Formula.parse('always[0, 4](x < 1)').falsify(m, ...
    sentil.Bounds([-2 -2 -2 -2], [2 2 2 2]), sentil.CmaConfig, 2);
verifyLessThan(t, w.robustness, 0);
delete(w.trace);
[p, ~] = sentil.Synthesis.maximize(@quadratic, [0 0], [], 500);
verifyEqual(t, p, [3 -1], 'AbsTol', 1e-3);
end

function testCustomSystemCallback(t)
init = @(seed) 0.0;
step = @(prev, time, seed) prev + (2 * double(mod(seed, 2)) - 1) * 0.6;
sys = sentil.StochasticSystem.custom({'x'}, 1.0, 8, init, step);
phi = sentil.Formula.parse('P>=0.99(always[0, 8](x < 5))');
r = phi.check_rare_event(sys, sentil.RareEventConfig);
verifyEqual(t, r.violation_probability, ...
    phi.check_rare_event(sys, sentil.RareEventConfig).violation_probability);
verifyGreaterThanOrEqual(t, r.violation_probability, 0);
end

function testParameterMining(t)
a = sentil.Trace([0 1 2], 'x', [1 3 2]);
b = sentil.Trace([0 1 2], 'x', [2 5 1]);
p = sentil.Synthesis.mine_tightest_parameter(...
    @(c) sentil.Formula.parse(sprintf('always[0,2](x < %g)', c)), [a b], 0, 10);
verifyEqual(t, p, 5, 'AbsTol', 1e-3);
end

function testSpecs(t)
names = sentil.SpecBuilder.available();
verifyEqual(t, numel(names), 54);
sb = sentil.SpecBuilder(names{1});
phi = sb.build_formula();
verifyNotEmpty(t, phi.variables());
end

function testErrors(t)
verifyError(t, @() sentil.Formula.parse('x >'), 'sentil:parse');
reg = sentil.LiftingRegistry();
reg.register('x', sentil.NoiseModel.gaussian(0, 1));
trace = sentil.Trace([0 1], 'x', [1 2]);
verifyError(t, @() sentil.Formula.parse('x > 0').check(trace, reg), 'sentil:semantic');
end

function [value, gradient] = quadratic(x)
value = -(x(1) - 3)^2 - (x(2) + 1)^2;
gradient = [-2 * (x(1) - 3), -2 * (x(2) + 1)];
end