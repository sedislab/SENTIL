using Sentil
using Printf

const FORMULA = "always[0, 100](eventually[0, 10](x > 5))"

now_ms() = time_ns() / 1.0e6

function summarize(samples::Vector{Float64})
    s = sort(samples)
    n = length(s)
    m = sum(s) / n
    v = n > 1 ? sum((x - m)^2 for x in s) / (n - 1) : 0.0
    pct(q) = s[clamp(round(Int, (n - 1) * q) + 1, 1, n)]
    return (mean = m, std = sqrt(v), min = s[1], p50 = pct(0.50), p99 = pct(0.99))
end

function cpu_model()
    info = Sys.cpu_info()
    isempty(info) ? "unknown" : strip(info[1].model)
end

function peak_rss_bytes()
    for line in eachline("/proc/self/status")
        startswith(line, "VmHWM:") && return parse(Int, split(line)[2]) * 1024
    end
    return -1
end

function emit(benchmark, formula, question, size, robustness, t, runs)
    rss = peak_rss_bytes()
    rss_field = rss >= 0 ? string(rss) : "null"
    @printf("{\"tool\":\"sentil\",\"version\":\"1.0.0\",\"language\":\"julia\",\"benchmark\":\"%s\",", benchmark)
    @printf("\"formula\":\"%s\",\"question\":\"%s\",\"size\":%d,\"robustness\":%.17g,", formula, question, size, robustness)
    @printf("\"timing\":{\"mean_ms\":%.17g,\"std_ms\":%.17g,\"min_ms\":%.17g,\"p50_ms\":%.17g,\"p99_ms\":%.17g},",
            t.mean, t.std, t.min, t.p50, t.p99)
    @printf("\"peak_rss_bytes\":%s,\"runs\":%d,\"hardware\":{\"cpu\":\"%s\",\"cores\":%d}}\n",
            rss_field, runs, cpu_model(), Sys.CPU_THREADS)
end

oracle_trace(n) = Trace(collect(0.0:(n - 1)), "x", [15.0 * sin(i * 0.1) for i in 0:(n - 1)])

function scalability()
    for n in (1000, 10000, 100000, 1000000, 10000000)
        runs = n <= 100000 ? 30 : 5
        trace = oracle_trace(n)
        monitor = Monitor(FORMULA)

        full_rob = robustness_signal(monitor, trace)[1]
        samples = Float64[]
        for _ in 1:runs
            t0 = now_ms()
            robustness_signal(monitor, trace)
            push!(samples, now_ms() - t0)
        end
        emit("scalability/length", FORMULA, "full_signal", n, full_rob, summarize(samples), runs)

        mon_rob = robustness(monitor, trace)
        samples = Float64[]
        for _ in 1:runs
            t0 = now_ms()
            robustness(monitor, trace)
            push!(samples, now_ms() - t0)
        end
        emit("scalability/length", FORMULA, "monitoring", n, mon_rob, summarize(samples), runs)
    end
end

function streaming()
    monitor = OnlineMonitor(FORMULA)
    index = symbol_index(monitor, "x")
    n = 1000000
    latencies = Vector{Float64}(undef, n)
    packed = zeros(1)
    last = 0.0
    for i in 0:(n - 1)
        packed[index] = 15.0 * sin(i * 0.1)
        t0 = now_ms()
        verdict = update_packed!(monitor, Float64(i), packed)
        latencies[i + 1] = now_ms() - t0
        last = verdict.lower
    end
    emit("streaming", FORMULA, "monitoring", n, last, summarize(latencies), n)
end

function main()
    suite = isempty(ARGS) ? "" : ARGS[1]
    if suite == "scalability"
        scalability()
    elseif suite == "streaming"
        streaming()
    else
        println(stderr, "unknown suite `$suite`; use `scalability` or `streaming`")
        exit(1)
    end
end

main()