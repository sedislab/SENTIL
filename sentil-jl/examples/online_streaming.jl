using Sentil

monitor = OnlineMonitor("always[0, 10] (x > -0.9)")
index = symbol_index(monitor, "x")

packed = zeros(1)
for t in 0:59
    packed[index] = sin(t * 0.3)
    verdict = update_packed!(monitor, Float64(t), packed)
    if verdict.resolved && !verdict.satisfied
        println("violated at t=", t, ", robustness=", round(verdict.value; digits = 3))
        break
    end
end