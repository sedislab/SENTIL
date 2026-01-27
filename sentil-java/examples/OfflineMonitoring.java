// Offline robustness over a recorded trace, in discrete and dense time.
//
//   javac -cp ../target/sentil-0.3.0.jar OfflineMonitoring.java
//   java -cp ../target/sentil-0.3.0.jar:. OfflineMonitoring
import io.github.sedislab.sentil.Formula;
import io.github.sedislab.sentil.Interval;
import io.github.sedislab.sentil.Trace;
import java.util.Arrays;

public class OfflineMonitoring {
    public static void main(String[] args) throws Exception {
        try (Trace trace = Trace.create(new double[] {0, 1, 2, 3, 4});
                Formula phi = Formula.parse("always (speed > 5)")) {
            trace.addSignal("speed", new double[] {12.0, 9.0, 7.0, 4.0, 6.0});

            System.out.println("robustness: " + phi.robustness(trace));
            System.out.println("per sample: " + Arrays.toString(phi.robustnessSignal(trace)));
            System.out.print("violations:");
            for (Interval v : phi.violations(trace)) {
                System.out.print(" [" + v.start() + ", " + v.end() + "]");
            }
            System.out.println();
            System.out.println("dense robustness: " + phi.robustnessDense(trace));
        }
    }
}