% Online streaming, folding one timestamped sample at a time.

monitor = sentil.OnlineMonitor('always[0, 10] (x > -0.9)');
for t = 0:59
    x = sin(t * 0.3);
    verdict = monitor.update(t, struct('x', x));
    if verdict.resolved && ~verdict.satisfied
        fprintf('violated at t=%d, robustness=%.3f\n', t, verdict.value);
        return
    end
end
fprintf('held over the whole stream\n');