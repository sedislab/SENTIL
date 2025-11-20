package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Arrays;
import org.junit.jupiter.api.Test;

class SignalTest {
    @Test
    void buildsAndReadsTraces() throws Exception {
        try (Trace t = Trace.create(new double[] {0, 1, 2})) {
            t.addSignal("x", new double[] {5, -3, 2});
            assertEquals(3, t.length());
            assertFalse(t.isEmpty());
            assertEquals(Arrays.asList("x"), t.variables());
            assertArrayEquals(new double[] {0, 1, 2}, t.times());
            assertArrayEquals(new double[] {5, -3, 2}, t.signal("x").orElseThrow(AssertionError::new));
            assertFalse(t.signal("y").isPresent());
        }
    }

    @Test
    void rejectsNonIncreasingTimes() {
        assertThrows(SentilException.class, () -> Trace.create(new double[] {0, 0, 1}).close());
    }

    @Test
    void resamplesAndParsesCsv() throws Exception {
        try (Trace t = Trace.create(new double[] {0, 2, 4})) {
            t.addSignal("x", new double[] {0, 2, 4});
            try (Trace r = t.resample(new double[] {0, 1, 2, 3, 4}, Interpolation.LINEAR)) {
                assertArrayEquals(new double[] {0, 1, 2, 3, 4},
                        r.signal("x").orElseThrow(AssertionError::new));
            }
            try (PreparedTrace p = t.prepare(Interpolation.LINEAR); Trace r = p.resample(new double[] {1, 3})) {
                assertArrayEquals(new double[] {1, 3}, r.signal("x").orElseThrow(AssertionError::new));
            }
        }
        try (Trace t = Trace.fromCsv("t,x\n0,5\n1,-3\n2,2\n")) {
            assertArrayEquals(new double[] {0, 1, 2}, t.times());
            assertArrayEquals(new double[] {5, -3, 2}, t.signal("x").orElseThrow(AssertionError::new));
        }
    }

    @Test
    void ringBufferEvictsAndAggregates() throws Exception {
        try (RingBuffer b = RingBuffer.create(3)) {
            assertTrue(b.isEmpty());
            assertFalse(b.push(0, 10).isPresent());
            b.push(1, 20);
            b.push(2, 30);
            assertTrue(b.isFull());
            assertEquals(10.0, b.push(3, 40).orElseThrow(AssertionError::new).value());
            assertEquals(20.0, b.front().value());
            assertEquals(40.0, b.back().value());
            assertEquals(30.0, b.closestToTime(2.4).value());
        }
        try (RingBuffer b = RingBuffer.create(5)) {
            b.push(0, 2);
            b.push(1, 4);
            b.push(2, 6);
            assertEquals(4.0, b.mean().orElseThrow(AssertionError::new));
            assertEquals(4.0, b.variance().orElseThrow(AssertionError::new));
            assertEquals(2.0, b.stdDev().orElseThrow(AssertionError::new));
            assertEquals(2.0, b.min().orElseThrow(AssertionError::new));
            assertEquals(6.0, b.max().orElseThrow(AssertionError::new));
            assertEquals(2, b.between(0.5, 2).size());
        }
    }
}