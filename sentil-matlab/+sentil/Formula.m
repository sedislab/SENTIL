classdef Formula < handle
    %FORMULA A parsed SENTIL temporal-logic formula.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function f = parse(text)
            %PARSE Parse a formula from its textual form.
            f = sentil.Formula(sentil_mex('formula_parse', text));
        end
    end

    methods
        function obj = Formula(handle)
            %FORMULA Wrap an engine handle.
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('formula_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function v = variables(obj)
            %VARIABLES The variable names the formula reads.
            obj.assertOpen();
            v = sentil_mex('formula_variables', obj.Handle);
        end

        function s = to_json(obj)
            %TO_JSON The formula as a JSON string.
            obj.assertOpen();
            s = sentil_mex('formula_to_json', obj.Handle);
        end

        function r = robustness(obj, trace)
            %ROBUSTNESS The robustness margin over a trace.
            obj.assertOpen();
            r = sentil_mex('formula_robustness', obj.Handle, trace.Handle);
        end

        function r = robustness_dense(obj, trace)
            %ROBUSTNESS_DENSE The dense-time robustness margin over a trace.
            obj.assertOpen();
            r = sentil_mex('formula_robustness_dense', obj.Handle, trace.Handle);
        end

        function r = robustness_signal(obj, trace)
            %ROBUSTNESS_SIGNAL The robustness at every sample, as a row vector.
            obj.assertOpen();
            r = sentil_mex('formula_robustness_signal', obj.Handle, trace.Handle);
        end

        function r = robustness_dense_signal(obj, trace)
            %ROBUSTNESS_DENSE_SIGNAL Dense robustness at every sample, as a row vector.
            obj.assertOpen();
            r = sentil_mex('formula_robustness_dense_signal', obj.Handle, trace.Handle);
        end

        function v = violations(obj, trace)
            %VIOLATIONS The time spans where the formula fails, as [start, end] rows.
            obj.assertOpen();
            v = sentil_mex('formula_violations', obj.Handle, trace.Handle);
        end

        function r = check(obj, trace, lifting, config)
            %CHECK Estimate the satisfaction probability of a P-wrapped formula.
            obj.assertOpen();
            if nargin < 4, config = sentil.SmcConfig; end
            r = sentil_mex('formula_check', obj.Handle, trace.Handle, lifting.Handle, ...
                config.samples, config.confidence, config.seed, double(int32(config.method)));
        end

        function r = check_conservative(obj, trace, lifting, config)
            %CHECK_CONSERVATIVE Estimate with the Clopper-Pearson interval.
            obj.assertOpen();
            if nargin < 4, config = sentil.SmcConfig; end
            r = sentil_mex('formula_check_conservative', obj.Handle, trace.Handle, ...
                lifting.Handle, config.samples, config.confidence, config.seed, ...
                double(int32(config.method)));
        end

        function [result, distribution] = check_distribution(obj, trace, lifting, config)
            %CHECK_DISTRIBUTION Estimate, returning the robustness distribution as well.
            obj.assertOpen();
            if nargin < 4, config = sentil.SmcConfig; end
            out = sentil_mex('formula_check_distribution', obj.Handle, trace.Handle, ...
                lifting.Handle, config.samples, config.confidence, config.seed, ...
                double(int32(config.method)));
            result = out.result;
            distribution = out.distribution;
        end

        function r = check_sequential(obj, trace, lifting, config)
            %CHECK_SEQUENTIAL Decide a P-wrapped formula by SPRT.
            obj.assertOpen();
            r = sentil_mex('formula_check_sequential', obj.Handle, trace.Handle, lifting.Handle, ...
                config.p0, config.p1, config.alpha, config.beta, config.max_samples, config.seed);
            r.verdict = sentil.SprtVerdict(r.verdict);
        end

        function r = check_bayesian(obj, trace, lifting, config)
            %CHECK_BAYESIAN Decide a P-wrapped formula by Bayesian sequential testing.
            obj.assertOpen();
            r = sentil_mex('formula_check_bayesian', obj.Handle, trace.Handle, lifting.Handle, ...
                config.threshold, config.bayes_factor, config.max_samples, config.seed);
            r.verdict = sentil.BayesVerdict(r.verdict);
        end
    end

    methods (Hidden)
        function h = consume(obj)
            %CONSUME Surrender the native handle, leaving this formula closed.
            obj.assertOpen();
            h = obj.Handle;
            obj.Handle = uint64(0);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this formula has been closed');
            end
        end
    end
end