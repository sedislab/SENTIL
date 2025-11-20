package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class NoiseTest {
    @Test
    void familiesReportMomentsAndRoundTrip() throws Exception {
        try (NoiseModel g = NoiseModel.gaussian(2.0, 3.0)) {
            assertEquals(2.0, g.mean().orElseThrow(AssertionError::new));
            assertEquals(9.0, g.variance().orElseThrow(AssertionError::new));
            try (NoiseModel back = NoiseModel.fromJson(g.toJson())) {
                assertEquals(2.0, back.mean().orElseThrow(AssertionError::new));
            }
        }
        try (NoiseModel u = NoiseModel.uniform(0, 10)) {
            assertEquals(5.0, u.mean().orElseThrow(AssertionError::new));
        }
        try (NoiseModel c = NoiseModel.cauchy(0, 1)) {
            assertFalse(c.mean().isPresent());
        }
    }

    @Test
    void fitsResidualsAndMixtures() throws Exception {
        double[] samples = {1.8, 2.1, 1.9, 2.2, 2.0, 1.95, 2.05};
        try (NoiseModel g = NoiseModel.fitGaussian(samples)) {
            assertEquals(2.0, g.mean().orElseThrow(AssertionError::new), 1e-9);
        }
        double[] residuals = NoiseModel.residuals(new double[] {1, 2, 3, 4},
                new double[] {1.1, 2.0, 3.2, 3.9}, NoiseInteraction.ADDITIVE);
        assertEquals(0.1, residuals[0], 1e-9);
        assertEquals(0.0, residuals[1], 1e-9);
        try (NoiseModel m = NoiseModel.mixture(new double[] {0.5, 0.5}, NoiseModel.gaussian(0, 1),
                NoiseModel.gaussian(10, 1))) {
            assertEquals(5.0, m.mean().orElseThrow(AssertionError::new), 1e-9);
        }
    }

    @Test
    void liftingIsSeedReproducible() throws Exception {
        try (LiftingRegistry reg = new LiftingRegistry()) {
            assertTrue(reg.isEmpty());
            reg.register("x", NoiseModel.gaussian(0, 0.5));
            assertFalse(reg.isEmpty());
            try (Trace t = Trace.create(new double[] {0, 1, 2})) {
                t.addSignal("x", new double[] {5, 5, 5});
                try (Trace a = reg.lift(t, 42); Trace b = reg.lift(t, 42)) {
                    assertEquals(a.signal("x").orElseThrow(AssertionError::new)[0],
                            b.signal("x").orElseThrow(AssertionError::new)[0]);
                }
            }
        }
    }
}