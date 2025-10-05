"""Online streaming: fold one timestamped sample at a time."""

import math

import sentil

monitor = sentil.OnlineMonitor("always[0, 10] (x > -0.9)")
for t in range(60):
    x = math.sin(t * 0.3)
    verdict = monitor.update(float(t), {"x": x})
    if verdict.resolved and not verdict.satisfied:
        print(f"violated at t={t}, robustness={verdict.value:.3f}")
        break
else:
    print("held over the whole stream")