function tests = test_oracle
%TEST_ORACLE Reproduces the robustness in benchmarks/deterministic/oracle.json.
tests = functiontests(localfunctions);
end

function testDeterministicOracle(testCase)
here = fileparts(mfilename('fullpath'));
oracle = jsondecode(fileread(fullfile(here, '..', '..', 'benchmarks', 'deterministic', ...
    'oracle.json')));
cases = oracle.deterministic;
verifyNotEmpty(testCase, cases);
for i = 1:numel(cases)
    c = element(cases, i);
    trace = sentil.Trace(0:(double(c.length) - 1));
    signals = c.signals;
    for j = 1:numel(signals)
        s = element(signals, j);
        trace.add_signal(s.name, parseTokens(s.values));
    end
    phi = sentil.Formula.parse(c.formula);
    got = phi.robustness_signal(trace);
    expected = parseTokens(c.expected);
    verifyEqual(testCase, numel(got), numel(expected), c.id);
    for k = 1:numel(expected)
        verifyTrue(testCase, bitEqual(got(k), expected(k)), ...
            sprintf('%s sample %d: got %.17g, expected %.17g', c.id, k, got(k), expected(k)));
    end
    delete(phi);
    delete(trace);
end
end

function e = element(arr, i)
if iscell(arr)
    e = arr{i};
else
    e = arr(i);
end
end

function v = parseTokens(tokens)
if ischar(tokens)
    tokens = {tokens};
elseif isstring(tokens)
    tokens = cellstr(tokens);
end
v = zeros(1, numel(tokens));
for i = 1:numel(tokens)
    t = tokens{i};
    switch t
        case 'inf'
            v(i) = Inf;
        case '-inf'
            v(i) = -Inf;
        case 'nan'
            v(i) = NaN;
        otherwise
            v(i) = str2double(t);
    end
end
end

function tf = bitEqual(got, expected)
if isnan(got) && isnan(expected)
    tf = true;
    return
end
tf = (got == expected) && (got ~= 0 || (1 / got) == (1 / expected));
end