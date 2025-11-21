package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.ByteBuffer;
import java.util.Arrays;
import org.junit.jupiter.api.Test;

class SynthesisTest {
    @Test
    void synthesizesAnInputForALinearModel() throws Exception {
        try (SystemModel m = SystemModel.linear(new double[][] {{1.0}}, new double[][] {{1.0}},
                new double[] {0.0}, new String[] {"x"}, 1.0, 5);
                Formula spec = Formula.parse("eventually[0,5](x > 5)");
                Bounds bounds = new Bounds(new double[] {-3, -3, -3, -3, -3},
                        new double[] {3, 3, 3, 3, 3})) {
            SynthesisResult r = Synthesis.synthesize(m, spec, bounds, new SmoothConfig(),
                    Backend.GRADIENT, 200, 0);
            assertTrue(r.holds());
            assertEquals(10.0, r.robustness(), 1e-6);
            assertArrayEquals(new double[] {3, 3, 3, 3, 3}, r.input(), 1e-6);
        }
    }

    @Test
    void numericsSolveAndDecompose() throws Exception {
        assertArrayEquals(new double[] {2, 3},
                Synthesis.solveSpd(new double[][] {{2, 0}, {0, 2}}, new double[] {4, 6}), 1e-9);
        EigenDecomposition eigen = Synthesis.symmetricEigen(new double[][] {{2, 0}, {0, 3}});
        assertArrayEquals(new double[] {2, 3}, eigen.values(), 1e-9);
    }

    @Test
    void boundsClampAndSafetyFilter() throws Exception {
        try (Bounds b = new Bounds(new double[] {-1, -1}, new double[] {1, 1})) {
            assertArrayEquals(new double[] {1, -1}, b.clamp(new double[] {5, -5}));
            try (SafetyFilter sf = new SafetyFilter(new Bounds(new double[] {-1, -1},
                    new double[] {1, 1}))) {
                assertArrayEquals(new double[] {1, -1}, sf.filter(new double[] {5, -5}), 1e-9);
            }
        }
    }

    @Test
    void witnessSearchFindsCounterexample() throws Exception {
        try (SystemModel m = SystemModel.linear(new double[][] {{1}}, new double[][] {{1}},
                new double[] {0}, new String[] {"x"}, 1.0, 4);
                Formula spec = Formula.parse("always[0,4](x < 1)");
                Bounds bounds = new Bounds(new double[] {-2, -2, -2, -2}, new double[] {2, 2, 2, 2})) {
            try (Witness w = spec.falsify(m, bounds, new CmaConfig().maxGenerations(100), 2)) {
                assertTrue(w.robustness() < 0);
            }
        }
    }

    @Test
    void optimizersClimbToThePeak() throws Exception {
        GradientObjective objective = (x, gradient) -> {
            gradient[0] = -2 * (x[0] - 3);
            gradient[1] = -2 * (x[1] + 1);
            return -(x[0] - 3) * (x[0] - 3) - (x[1] + 1) * (x[1] + 1);
        };
        Optimum maximized = Synthesis.maximize(objective, new double[] {0, 0}, null, 500);
        assertArrayEquals(new double[] {3, -1}, maximized.point(), 1e-3);
        Optimum searched = Synthesis.cmaEs(x -> -(x[0] - 3) * (x[0] - 3) - (x[1] + 1) * (x[1] + 1),
                new double[] {0, 0}, null, new CmaConfig().maxGenerations(200));
        assertArrayEquals(new double[] {3, -1}, searched.point(), 1e-2);
    }

    @Test
    void minesTheTightestParameter() throws Exception {
        try (Trace a = Trace.create(new double[] {0, 1, 2}); Trace b = Trace.create(new double[] {0, 1, 2})) {
            a.addSignal("x", new double[] {1, 3, 2});
            b.addSignal("x", new double[] {2, 5, 1});
            double tightest = Synthesis.mineTightestParameter(
                    c -> Formula.parse("always[0,2](x < " + c + ")"), Arrays.asList(a, b), 0.0, 10.0);
            assertEquals(5.0, tightest, 1e-3);
        }
    }

    @Test
    void hostSystemDrivesChanceValidation() throws Exception {
        SystemInit init = seed -> new double[] {0.0};
        SystemStep step = (prev, t, seed) -> new double[] {prev[0] + ((seed & 1L) == 0 ? -0.4 : 0.6)};
        try (StochasticSystem sys = StochasticSystem.custom(new String[] {"x"}, 1.0, 8, init, step);
                Formula spec = Formula.parse("always[0,8](x < 5)");
                ChanceConstraint cc = new ChanceConstraint(spec, 0.5, 0.95, 0.0)) {
            ChanceReport r = cc.validate(sys, 400, 7);
            System.gc();
            assertEquals(r.estimate(), cc.validate(sys, 400, 7).estimate());
            assertTrue(r.lowerBound() <= r.estimate() && r.estimate() <= 1.0);
        }
        try (StochasticSystem bad = StochasticSystem.custom(new String[] {"x"}, 1.0, 4,
                seed -> new double[] {0.0}, (prev, t, seed) -> {
                    throw new IllegalStateException("step boom");
                });
                Formula spec = Formula.parse("always (x < 5)");
                ChanceConstraint cc = new ChanceConstraint(spec, 0.5)) {
            IllegalStateException e =
                    assertThrows(IllegalStateException.class, () -> cc.validate(bad, 50, 1));
            assertTrue(e.getMessage().contains("step boom"));
        }
    }

    @Test
    void customModelDrivesAControllerWithoutLeaking() throws Exception {
        Rollout rollout = (initial, input) -> {
            double[] row = new double[input.length + 1];
            row[0] = initial[0];
            for (int i = 0; i < input.length; i++) {
                row[i + 1] = row[i] + input[i];
            }
            return new double[][] {row};
        };
        for (int run = 0; run < 3; run++) {
            try (SystemModel model = SystemModel.custom(new String[] {"x"}, 1.0, 3, new double[] {0.0},
                    1, rollout);
                    Formula spec = Formula.parse("always[0,3](x < 10)");
                    Controller controller = new Controller(model, spec, 1, 1_000_000_000L,
                            new Bounds(new double[] {-2, -2, -2}, new double[] {2, 2, 2}), null)) {
                double[] u = controller.control(new double[] {0.0});
                assertEquals(1, u.length);
                assertTrue(-2.0 - 1e-6 <= u[0] && u[0] <= 2.0 + 1e-6);
            }
            System.gc();
        }
    }

    @Test
    void adaptiveMultilevelSplittingOverAHostSimulator() throws Exception {
        AmsInterface walk = new AmsInterface() {
            public int stateSize() {
                return 8;
            }

            public byte[] initialState(long seed) {
                return ByteBuffer.allocate(8).putDouble(1.0).array();
            }

            public byte[] step(byte[] state, long seed) {
                double x = ByteBuffer.wrap(state).getDouble();
                return ByteBuffer.allocate(8).putDouble(x + ((seed & 1L) == 0 ? -0.5 : 0.5)).array();
            }

            public boolean isTerminal(byte[] state, boolean[] inRareEvent) {
                double x = ByteBuffer.wrap(state).getDouble();
                inRareEvent[0] = x >= 5.0;
                return x >= 5.0 || x <= 0.0;
            }

            public double score(byte[] state) {
                return ByteBuffer.wrap(state).getDouble();
            }
        };
        RareEventEstimate estimate = Synthesis.adaptiveMultilevelSplitting(walk, 2000, 5.0, 2000, 42);
        assertTrue(estimate.probability() > 0.0 && estimate.probability() < 1.0);
        assertTrue(estimate.simulations() > 0);
        assertEquals(estimate.probability(),
                Synthesis.adaptiveMultilevelSplitting(walk, 2000, 5.0, 2000, 42).probability());
    }
}