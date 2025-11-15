package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import org.junit.jupiter.api.Test;

class OracleTest {
    private static double token(String text) {
        switch (text) {
            case "inf":
                return Double.POSITIVE_INFINITY;
            case "-inf":
                return Double.NEGATIVE_INFINITY;
            case "nan":
                return Double.NaN;
            default:
                return Double.parseDouble(text);
        }
    }

    private static double[] values(Json array) {
        double[] out = new double[array.size()];
        for (int i = 0; i < out.length; i++) {
            out[i] = token(array.get(i).str());
        }
        return out;
    }

    @Test
    void reproducesDeterministicOracle() throws Exception {
        Path path = Paths.get(
                System.getProperty("sentil.oracle", "../benchmarks/deterministic/oracle.json"));
        Json root = Json.parse(new String(Files.readAllBytes(path), StandardCharsets.UTF_8));
        Json cases = root.get("deterministic");

        int reproduced = 0;
        for (int c = 0; c < cases.size(); c++) {
            Json test = cases.get(c);
            String id = test.get("id").str();
            int length = test.get("length").asInt();
            try (Trace trace = Trace.indexed(length);
                    Formula phi = Formula.parse(test.get("formula").str())) {
                Json signals = test.get("signals");
                for (int s = 0; s < signals.size(); s++) {
                    Json signal = signals.get(s);
                    trace.addSignal(signal.get("name").str(), values(signal.get("values")));
                }
                double[] expected = values(test.get("expected"));
                double[] got = phi.robustnessSignal(trace);
                assertEquals(expected.length, got.length, id + " signal length");
                for (int i = 0; i < got.length; i++) {
                    long want = Double.doubleToRawLongBits(expected[i]);
                    long have = Double.doubleToRawLongBits(got[i]);
                    int sample = i;
                    assertEquals(want, have,
                            () -> id + " at sample " + sample + ": got " + got[sample]);
                }
            }
            reproduced++;
        }
        assertTrue(reproduced >= 44, "reproduced only " + reproduced + " deterministic cases");
    }
}