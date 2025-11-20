package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class ErrorsTest {
    @Test
    void parseErrorCarriesTheColumn() {
        ParseException e = assertThrows(ParseException.class, () -> Formula.parse("x > "));
        assertEquals(ErrorCode.PARSE, e.errorCode());
        assertFalse(e.getMessage().isEmpty());
    }

    @Test
    void nonProbabilisticFormulaIsSemantic() throws Exception {
        try (Formula f = Formula.parse("x > 0"); Trace t = Trace.create(new double[] {0, 1});
                LiftingRegistry reg = new LiftingRegistry()) {
            t.addSignal("x", new double[] {1, 2});
            reg.register("x", NoiseModel.gaussian(0, 1));
            SemanticException e = assertThrows(SemanticException.class, () -> f.check(t, reg));
            assertEquals(ErrorCode.NOT_PROBABILISTIC, e.errorCode());
        }
    }

    @Test
    void unknownVariableIsSemantic() throws Exception {
        try (Formula f = Formula.parse("y > 0"); Trace t = Trace.create(new double[] {0, 1})) {
            t.addSignal("x", new double[] {1, 2});
            SemanticException e = assertThrows(SemanticException.class, () -> f.robustness(t));
            assertEquals(ErrorCode.UNKNOWN_VARIABLE, e.errorCode());
        }
    }

    @Test
    void invertedBoundsAreRejected() {
        assertThrows(SentilException.class,
                () -> new Bounds(new double[] {1, 1}, new double[] {-1, -1}).close());
    }
}