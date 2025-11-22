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

        function r = check_rare_event(obj, system, config)
            %CHECK_RARE_EVENT Estimate a P-wrapped formula by adaptive multilevel splitting.
            obj.assertOpen();
            if nargin < 3, config = sentil.RareEventConfig; end
            r = sentil_mex('formula_check_rare_event', obj.Handle, system.Handle, system.Box, ...
                config.particles, config.margin, config.seed);
        end

        function w = find_counterexample(obj, model, bounds, maxIters, smooth)
            %FIND_COUNTEREXAMPLE Search the smooth robustness for a witnessing input.
            obj.assertOpen();
            if nargin < 4, maxIters = 0; end
            if nargin < 5, smooth = sentil.SmoothConfig; end
            out = sentil_mex('formula_find_counterexample', obj.Handle, model.Handle, ...
                bounds.Handle, double(maxIters), smooth.temperature, double(int32(smooth.kind)));
            w = obj.wrapWitness(out);
        end

        function w = falsify(obj, model, bounds, config, restarts)
            %FALSIFY Minimize exact robustness with restarted CMA-ES.
            obj.assertOpen();
            if nargin < 4, config = sentil.CmaConfig; end
            if nargin < 5, restarts = 1; end
            out = sentil_mex('formula_falsify', obj.Handle, model.Handle, bounds.Handle, ...
                config.population, config.max_generations, config.initial_step, config.tol_step, ...
                config.seed, double(restarts));
            w = obj.wrapWitness(out);
        end

        function g = smooth_gradient(obj, model, initial, input, smooth)
            %SMOOTH_GRADIENT The smooth robustness of a rollout and its gradient per input.
            obj.assertOpen();
            if nargin < 5, smooth = sentil.SmoothConfig; end
            g = sentil_mex('formula_smooth_gradient', obj.Handle, model.Handle, double(initial), ...
                double(input), smooth.temperature, double(int32(smooth.kind)));
        end

        function g = smooth_value_and_gradient(obj, trace, smooth)
            %SMOOTH_VALUE_AND_GRADIENT The smooth robustness over a trace and its gradient.
            obj.assertOpen();
            if nargin < 3, smooth = sentil.SmoothConfig; end
            g = sentil_mex('formula_smooth_value_and_gradient', obj.Handle, trace.Handle, ...
                smooth.temperature, double(int32(smooth.kind)));
        end
    end

    methods (Static, Access = private)
        function w = wrapWitness(out)
            w = struct('input', out.input, 'robustness', out.robustness, ...
                'trace', sentil.Trace(uint64(out.trace)));
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