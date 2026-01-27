// Online streaming: fold one timestamped sample at a time.
//
//   javac -cp ../target/sentil-0.3.0.jar OnlineStreaming.java
//   java -cp ../target/sentil-0.3.0.jar:. OnlineStreaming
import io.github.sedislab.sentil.OnlineMonitor;
import io.github.sedislab.sentil.Robustness;
import java.util.Collections;

public class OnlineStreaming {
    public static void main(String[] args) throws Exception {
        try (OnlineMonitor monitor = OnlineMonitor.create("always[0, 10] (x > -0.9)")) {
            for (int t = 0; t < 60; t++) {
                double x = Math.sin(t * 0.3);
                Robustness verdict = monitor.update(t, Collections.singletonMap("x", x));
                if (verdict.resolved() && !verdict.satisfied()) {
                    System.out.printf("violated at t=%d, robustness=%.3f%n", t, verdict.value());
                    return;
                }
            }
            System.out.println("held over the whole stream");
        }
    }
}