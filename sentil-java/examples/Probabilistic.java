// Probabilistic monitoring: lift a noisy sensor and estimate satisfaction.
//
//   javac -cp ../target/sentil-1.0.0.jar Probabilistic.java
//   java -cp ../target/sentil-1.0.0.jar:. Probabilistic
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.LiftingRegistry;
import io.github.sedislab.sentil.NoiseModel;
import io.github.sedislab.sentil.SmcConfig;
import io.github.sedislab.sentil.SmcResult;
import io.github.sedislab.sentil.Trace;

public class Probabilistic {
    public static void main(String[] args) throws Exception {
        double[] times = new double[20];
        double[] values = new double[20];
        for (int i = 0; i < 20; i++) {
            times[i] = i;
            values[i] = 0.4 + 0.05 * i;
        }
        try (Trace trace = Trace.create(times);
                LiftingRegistry lifting = new LiftingRegistry();
                Formula phi = Formula.parse("P>=0.9 (always (x > 0))")) {
            trace.addSignal("x", values);
            lifting.register("x", NoiseModel.gaussian(0.0, 0.3));

            SmcResult result = phi.check(trace, lifting, new SmcConfig().samples(5000));
            System.out.printf("probability %.3f, interval [%.3f, %.3f], holds %b%n",
                    result.probability(), result.interval().lower(), result.interval().upper(),
                    result.holds());
        }
    }
}