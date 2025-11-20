package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class StatsTest {
    @Test
    void confidenceIntervalsMatchReferenceValues() throws Exception {
        ConfidenceInterval wilson = Stats.wilson(50, 100, 0.95);
        assertEquals(0.403832, wilson.lower(), 1e-6);
        assertEquals(0.596168, wilson.upper(), 1e-6);
        ConfidenceInterval cp = Stats.clopperPearson(50, 100, 0.95);
        assertEquals(0.398321, cp.lower(), 1e-6);
        assertEquals(0.601679, cp.upper(), 1e-6);
        assertEquals(1.95996, Stats.zScore(0.95), 1e-5);
        assertEquals(185, Stats.chernoffHoeffdingSamples(0.1, 0.05));
    }

    @Test
    void smcEstimatesAndReproduces() throws Exception {
        try (Formula f = Formula.parse("P>=0.5(eventually[0,4](x > 2))");
                Trace t = Trace.create(new double[] {0, 1, 2, 3, 4});
                LiftingRegistry reg = new LiftingRegistry()) {
            t.addSignal("x", new double[] {3, 3, 3, 3, 3});
            reg.register("x", NoiseModel.gaussian(0, 1.0));
            SmcResult r = f.check(t, reg);
            assertTrue(r.holds());
            assertTrue(r.probability() > 0.99);
            assertEquals(10000, r.samples());
            assertEquals(r.probability(), f.check(t, reg).probability());
            SmcResult conservative = f.checkConservative(t, reg);
            assertTrue(conservative.interval().width() >= r.interval().width() - 1e-9);
        }
    }

    @Test
    void sequentialTestsDecideEarly() throws Exception {
        try (Formula f = Formula.parse("P>=0.5(eventually[0,4](x > 2))");
                Trace t = Trace.create(new double[] {0, 1, 2, 3, 4});
                LiftingRegistry reg = new LiftingRegistry()) {
            t.addSignal("x", new double[] {3, 3, 3, 3, 3});
            reg.register("x", NoiseModel.gaussian(0, 1.0));
            SprtResult sprt = f.checkSequential(t, reg, new SprtConfig(0.4, 0.6));
            assertEquals(SprtVerdict.ACCEPT_H1, sprt.verdict());
            assertTrue(sprt.samples() < 10000);
            BayesResult bayes = f.checkBayesian(t, reg, new BayesConfig(0.5));
            assertEquals(BayesVerdict.HOLDS, bayes.verdict());
        }
    }
}