import pytest

import sentil
from sentil import (Formula, LiftingRegistry, NoiseModel, NoiseInteraction, SmcConfig,
                    SprtConfig, BayesConfig, SprtVerdict, BayesVerdict, SimExpr, SimModel,
                    RareEventConfig, stats)

def test_confidence_oracle_numbers():
    w = stats.wilson_interval(50, 100, 0.95)
    assert w.lower == pytest.approx(0.403831, abs=1e-5)
    assert w.upper == pytest.approx(0.596169, abs=1e-5)
    cp = stats.clopper_pearson(50, 100, 0.95)
    assert cp.lower == pytest.approx(0.398321, abs=1e-5)
    assert stats.z_score(0.95) == pytest.approx(1.95996, abs=1e-5)
    assert stats.chernoff_hoeffding_samples(0.1, 0.05) == 185

def test_noise_models():
    g = NoiseModel.gaussian(2.0, 0.5)
    assert g.mean() == pytest.approx(2.0) and g.variance() == pytest.approx(0.25)
    assert NoiseModel.cauchy(0.0, 1.0).mean() is None
    assert NoiseModel.from_json(g.to_json()).mean() == g.mean()
    res = NoiseModel.residuals([1.0, 2.0, 3.0], [1.1, 1.9, 3.2], NoiseInteraction.Additive)
    assert len(res) == 3
    assert NoiseModel.fit_gaussian([0.1, -0.2, 0.05, 0.3]).mean() is not None

def lifted():
    trace = sentil.Trace([0, 1, 2, 3], {"x": [0.5, 0.4, 0.6, 0.55]})
    lift = LiftingRegistry()
    lift.register("x", NoiseModel.gaussian(0.0, 0.2))
    return trace, lift

def test_smc_check():
    trace, lift = lifted()
    phi = Formula.parse("P>=0.5 (always (x > 0))")
    result = phi.check(trace, lift, SmcConfig(samples=500))
    assert 0.0 <= result.probability <= 1.0
    assert result.interval.lower <= result.interval.upper
    _, dist = phi.check_distribution(trace, lift, SmcConfig(samples=500))
    assert dist.count > 0 and dist.std_dev >= 0.0

def test_non_probabilistic_check_raises():
    trace, lift = lifted()
    with pytest.raises(sentil.SemanticError):
        Formula.parse("always (x > 0)").check(trace, lift)

def test_sequential_tests():
    trace, lift = lifted()
    spec = Formula.parse("P>=0.5 (always (x > 0))")
    sprt = spec.check_sequential(trace, lift, SprtConfig(0.4, 0.6, max_samples=2000))
    assert sprt.verdict in (SprtVerdict.AcceptH0, SprtVerdict.AcceptH1, SprtVerdict.Inconclusive)
    bayes = spec.check_bayesian(trace, lift, BayesConfig(0.5, max_samples=2000))
    assert bayes.verdict in (BayesVerdict.Holds, BayesVerdict.Fails, BayesVerdict.Inconclusive)

def test_sim_model_and_rare_event():
    walk = SimModel(["y"], 0.1, 16, [SimExpr.constant(0.0)],
                    [SimExpr.prev(0) + SimExpr.noise(0)], [NoiseModel.gaussian(0, 1)])
    assert len(walk.simulate(seed=7)) == 17
    system = walk.to_stochastic_system()
    phi = Formula.parse("P>=0.99 (always[0,3] (y < 5))")
    rare = phi.check_rare_event(system, RareEventConfig(particles=256))
    assert 0.0 <= rare.violation_probability <= 1.0 and rare.simulations > 0