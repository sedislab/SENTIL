// Synthesize a control input that satisfies a spec, then shield it online.
//
//   javac -cp ../target/sentil-1.0.0.jar Synthesis.java
//   java -cp ../target/sentil-1.0.0.jar:. Synthesis
import io.github.sedislab.sentil.Backend;
import io.github.sedislab.sentil.Bounds;
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.SafetyFilter;
import io.github.sedislab.sentil.SynthesisResult;
import io.github.sedislab.sentil.SystemModel;
import java.util.Arrays;

public class Synthesis {
    public static void main(String[] args) throws Exception {
        // x_{t+1} = x_t + u_t
        try (SystemModel model = SystemModel.linear(new double[][] {{1.0}}, new double[][] {{1.0}},
                new double[] {1.0}, new String[] {"x"}, 1.0, 3);
                Formula spec = Formula.parse("always (x > 0)");
                Bounds bounds = new Bounds(new double[] {-1, -1, -1}, new double[] {1, 1, 1})) {
            SynthesisResult result = io.github.sedislab.sentil.Synthesis.synthesize(model, spec,
                    bounds, null, Backend.AUTO, 0, 0);
            System.out.println("input: " + Arrays.toString(result.input()) + " robustness: "
                    + result.robustness() + " holds: " + result.holds());
        }

        try (Bounds bounds = new Bounds(new double[] {-1, -1, -1}, new double[] {1, 1, 1});
                SafetyFilter shield = new SafetyFilter(bounds)) {
            System.out.println("shielded: " + Arrays.toString(shield.filter(new double[] {2.0, 0.5, -3.0})));
        }
    }
}