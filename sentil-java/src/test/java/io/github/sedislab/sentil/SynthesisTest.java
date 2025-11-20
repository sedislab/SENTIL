package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

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
}