function breach_runner(suite)
% Breach baseline
if exist('InitBreach', 'file') ~= 2
    breach_dir = getenv('BREACH_DIR');
    if isempty(breach_dir)
        error('breach:path', 'add Breach to the path or set BREACH_DIR');
    end
    addpath(breach_dir);
end
InitBreach;

canonical = {
    'always[0, 10](x < 5)',                                   'alw_[0,10] (x[t] < 5)'
    'eventually[0, 50](x > 10)',                              'ev_[0,50] (x[t] > 10)'
    'always[0, 100](eventually[0, 10](p > 0))',              'alw_[0,100] (ev_[0,10] (p[t] > 0))'
    '(p > 0) implies (eventually[0, 20](q > 0))',            '(p[t] > 0) => (ev_[0,20] (q[t] > 0))'
    'always[0, 200]((p > 0) and (eventually[5, 15](q > 0)))', 'alw_[0,200] ((p[t] > 0) and (ev_[5,15] (q[t] > 0)))'
};
scalability_sentil = 'always[0, 100](eventually[0, 10](x > 5))';
scalability_breach = 'alw_[0,100] (ev_[0,10] (x[t] > 5))';

switch suite
    case 'deterministic'
        for i = 1:size(canonical, 1)
            print_record('deterministic', canonical{i, 1}, canonical{i, 2}, 2001, 50);
        end
    case 'scalability'
        for n = [1000, 10000, 100000, 1000000]
            runs = 30; if n > 100000, runs = 5; end
            print_record('scalability/length', scalability_sentil, scalability_breach, n, runs);
        end
    otherwise
        error('breach:suite', 'unknown suite; use deterministic or scalability');
end
end

function print_record(benchmark, sentil_formula, breach_formula, n, runs)
[t, x, p, q] = signals(n);
phi = STL_Formula('phi', breach_formula);
B = BreachTraceSystem({'x', 'p', 'q'});
B.AddTrace([t(:), x(:), p(:), q(:)]);
robustness = B.CheckSpec(phi);
times_ms = zeros(1, runs);
for r = 1:runs
    tic;
    B.CheckSpec(phi);
    times_ms(r) = toc * 1e3;
end
times_ms = sort(times_ms);
last = numel(times_ms);
timing = struct( ...
    'mean_ms', mean(times_ms), ...
    'std_ms', std_or_zero(times_ms), ...
    'min_ms', times_ms(1), ...
    'p50_ms', times_ms(max(1, round(last * 0.50))), ...
    'p99_ms', times_ms(max(1, round(last * 0.99))));
record = struct( ...
    'tool', 'breach', ...
    'version', breach_version(), ...
    'language', 'matlab', ...
    'benchmark', benchmark, ...
    'formula', sentil_formula, ...
    'question', 'monitoring', ...
    'size', uint64(n), ...
    'robustness', robustness, ...
    'timing', timing, ...
    'peak_rss_bytes', NaN, ...
    'runs', uint64(runs), ...
    'hardware', hardware());
fprintf('%s\n', jsonencode(record));
end

function [t, x, p, q] = signals(n)
i = 0:(n - 1);
x = 15.0 * sin(0.1 * i);
p = 1.0 - 2.0 * mod(floor(i / 10), 2);
q = ones(1, n);
t = double(i);
end

function s = std_or_zero(v)
if numel(v) > 1, s = std(v); else, s = 0.0; end
end

function v = breach_version()
v = '1.11.0';
if exist('BreachVersion', 'file') == 2
    try, v = BreachVersion(); catch, end
end
end

function h = hardware()
cpu = 'unknown';
try
    info = fileread('/proc/cpuinfo');
    tokens = regexp(info, 'model name\s*:\s*(.*?)\n', 'tokens', 'once');
    if ~isempty(tokens), cpu = strtrim(tokens{1}); end
catch
end
h = struct('cpu', cpu, 'cores', feature('numcores'));
end