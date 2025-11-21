package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

class MonitorTest {
    @Test
    void offlineMonitorEvaluatesAndUpdates() throws Exception {
        try (Monitor m = Monitor.parse("always[0,2](x > 0)");
                Trace t = Trace.create(new double[] {0, 1, 2, 3})) {
            t.addSignal("x", new double[] {1, 2, -1, 3});
            assertEquals(-1.0, m.robustness(t));
            List<Interval> violations = m.violations(t);
            assertEquals(1, violations.size());
            assertEquals(0.0, violations.get(0).start());
            assertEquals(2.0, violations.get(0).end());
            assertEquals(0L, m.symbolIndex("x").orElseThrow(AssertionError::new));
            assertFalse(m.symbolIndex("y").isPresent());
        }
        try (Monitor m = Monitor.parse("x > 0")) {
            assertEquals(2.0, m.update(0, Collections.singletonMap("x", 2.0)).value());
            assertEquals(-3.0, m.updatePacked(1, new double[] {-3.0}).value());
        }
    }

    @Test
    void streamingRunMatchesUpdates() throws Exception {
        try (OnlineMonitor m = OnlineMonitor.create("x > 0");
                Trace t = Trace.create(new double[] {0, 1, 2})) {
            t.addSignal("x", new double[] {2, -1, 3});
            List<Robustness> run = m.run(t);
            assertEquals(3, run.size());
            assertEquals(2.0, run.get(0).value());
            assertEquals(-1.0, run.get(1).value());
        }
        try (OnlineMonitor m = OnlineMonitor.create("x > 0")) {
            assertEquals(5.0, m.updatePacked(0, new double[] {5}).value());
            m.reset();
            assertEquals(3.0, m.updatePacked(0, new double[] {3}).value());
        }
    }

    @Test
    void multiMonitorAndBankKeepResultsById() throws Exception {
        try (MultiMonitor m = new MultiMonitor()) {
            m.add("safety", "x > 0");
            try (Formula f = Formula.parse("x < 10")) {
                m.add("bound", f);
            }
            assertEquals(2, m.size());
            Map<String, Robustness> r = m.update(0, Collections.singletonMap("x", 5.0));
            assertEquals(5.0, r.get("safety").value());
            assertEquals(5.0, r.get("bound").value());
            assertTrue(m.remove("bound"));
            assertEquals(1, m.size());
        }
        try (FormulaBank b = new FormulaBank(); Trace t = Trace.create(new double[] {0, 1, 2})) {
            t.addSignal("x", new double[] {3, -1, 4});
            b.add("p1", "always(x > 0)");
            assertEquals(-1.0, b.robustness(t).get("p1"));
            b.add("bad", "x > q");
            SentilException e = assertThrows(SentilException.class, () -> b.robustness(t));
            assertTrue(e.getMessage().contains("bad"));
        }
    }
}