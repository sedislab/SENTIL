package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Arrays;
import org.junit.jupiter.api.Test;

class FormulaTest {
    @Test
    void parsesIntrospectsAndRoundTrips() throws Exception {
        try (Formula f = Formula.parse("always[0,10](x > 0 and y < 5)")) {
            assertEquals(Arrays.asList("x", "y"), f.variables());
            assertEquals(3, f.depth());
            assertTrue(f.hasTemporal());
            try (Formula back = Formula.fromJson(f.toJson())) {
                assertEquals(f.variables(), back.variables());
                assertEquals(f.depth(), back.depth());
            }
        }
        try (Formula c = Formula.parse("1 > 0")) {
            assertTrue(c.variables().isEmpty());
            assertFalse(c.hasTemporal());
        }
    }

    @Test
    void evaluatesRobustness() throws Exception {
        try (Formula f = Formula.parse("x > 0"); Trace t = Trace.create(new double[] {0, 1, 2})) {
            t.addSignal("x", new double[] {5, -3, 2});
            assertArrayEquals(new double[] {5, -3, 2}, f.robustnessSignal(t));
            assertEquals(5.0, f.robustness(t));
        }
        try (Formula f = Formula.parse("always[0,2](x > 0)");
                Trace t = Trace.create(new double[] {0, 1, 2, 3})) {
            t.addSignal("x", new double[] {1, 2, -1, 3});
            assertEquals(-1.0, f.robustness(t));
            assertEquals(Arrays.asList(new Interval(0.0, 2.0).toString()),
                    Arrays.asList(f.violations(t).get(0).toString()));
        }
    }

    @Test
    void builtFormulasMatchParsed() throws Exception {
        try (Trace t = Trace.create(new double[] {0, 1, 2})) {
            t.addSignal("x", new double[] {3, -1, 4});
            t.addSignal("y", new double[] {1, 2, 3});
            try (Formula built = Expr.var("x").mul(2).gt(5); Formula parsed = Formula.parse("x * 2 > 5")) {
                assertArrayEquals(parsed.robustnessSignal(t), built.robustnessSignal(t));
            }
            try (Formula built = Expr.var("x").gt(0).eventually(0, 1).always(0, 2);
                    Formula parsed = Formula.parse("always[0,2](eventually[0,1](x > 0))")) {
                assertArrayEquals(parsed.robustnessSignal(t), built.robustnessSignal(t));
            }
        }
    }

    @Test
    void denseTimeCatchesCrossings() throws Exception {
        try (Formula f = Formula.parse("eventually[0,1](x > 5)");
                Trace t = Trace.create(new double[] {0, 1})) {
            t.addSignal("x", new double[] {0, 10});
            assertEquals(5.0, f.robustnessDense(t));
        }
    }
}