function sentil_runner(suite)

formula = 'always[0, 100](eventually[0, 10](x > 5))';
switch suite
    case 'scalability'
        scalability(formula);
    case 'streaming'
        streaming(formula);
    otherwise
        error('sentil_runner:suite', 'unknown suite "%s"; use scalability or streaming', suite);
end
end

function scalability(formula)
for n = [1000 10000 100000 1000000 10000000]
    runs = 5;
    if n <= 100000, runs = 30; end
    trace = oracle_trace(n);
    monitor = sentil.Monitor(formula);

    full_rob = monitor.robustness_signal(trace);
    samples = zeros(1, runs);
    for r = 1:runs
        t0 = tic;
        monitor.robustness_signal(trace);
        samples(r) = toc(t0) * 1000;
    end
    emit('scalability/length', formula, 'full_signal', n, full_rob(1), summarize(samples), runs);

    mon_rob = monitor.robustness(trace);
    samples = zeros(1, runs);
    for r = 1:runs
        t0 = tic;
        monitor.robustness(trace);
        samples(r) = toc(t0) * 1000;
    end
    emit('scalability/length', formula, 'monitoring', n, mon_rob, summarize(samples), runs);

    delete(monitor);
    delete(trace);
end
end

function streaming(formula)
monitor = sentil.OnlineMonitor(formula);
index = monitor.symbol_index('x');
n = 1000000;
latencies = zeros(1, n);
packed = zeros(1, monitor.variable_count());
last = 0.0;
for i = 0:(n - 1)
    packed(index) = 15.0 * sin(i * 0.1);
    t0 = tic;
    verdict = monitor.update_packed(i, packed);
    latencies(i + 1) = toc(t0) * 1000;
    last = verdict.lower;
end
emit('streaming', formula, 'monitoring', n, last, summarize(latencies), n);
delete(monitor);
end

function trace = oracle_trace(n)
times = 0:(n - 1);
trace = sentil.Trace(times, 'x', 15.0 * sin(times * 0.1));
end

function s = summarize(samples)
v = sort(samples);
n = numel(v);
s.mean = mean(v);
if n > 1, s.std = std(v); else, s.std = 0.0; end
s.min = v(1);
s.p50 = v(min(max(round((n - 1) * 0.50) + 1, 1), n));
s.p99 = v(min(max(round((n - 1) * 0.99) + 1, 1), n));
end

function emit(benchmark, formula, question, size, robustness, t, runs)
rss = peak_rss_bytes();
if rss >= 0, rss_field = sprintf('%d', rss); else, rss_field = 'null'; end
fprintf('{"tool":"sentil","version":"1.0.0","language":"matlab","benchmark":"%s",', benchmark);
fprintf('"formula":"%s","question":"%s","size":%d,"robustness":%.17g,', formula, question, size, robustness);
fprintf('"timing":{"mean_ms":%.17g,"std_ms":%.17g,"min_ms":%.17g,"p50_ms":%.17g,"p99_ms":%.17g},', ...
    t.mean, t.std, t.min, t.p50, t.p99);
fprintf('"peak_rss_bytes":%s,"runs":%d,"hardware":{"cpu":"%s","cores":%d}}\n', ...
    rss_field, runs, cpu_model(), cpu_cores());
end

function bytes = peak_rss_bytes()
bytes = -1;
try
    text = fileread('/proc/self/status');
    token = regexp(text, 'VmHWM:\s*(\d+)\s*kB', 'tokens', 'once');
    if ~isempty(token)
        bytes = str2double(token{1}) * 1024;
    end
catch
end
end

function name = cpu_model()
name = 'unknown';
try
    text = fileread('/proc/cpuinfo');
    token = regexp(text, 'model name\s*:\s*([^\n]+)', 'tokens', 'once');
    if ~isempty(token)
        name = strtrim(token{1});
    end
catch
end
end

function n = cpu_cores()
n = feature('numcores');
try
    text = fileread('/proc/cpuinfo');
    n = numel(regexp(text, 'processor\s*:', 'start'));
catch
end
end