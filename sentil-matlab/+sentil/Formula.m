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

        function f = from_json(json)
            %FROM_JSON Build a formula from its JSON form.
            f = sentil.Formula(sentil_mex('formula_from_json', char(json)));
        end

        function f = predicate(lhs, op, rhs)
            %PREDICATE A predicate lhs op rhs. The operands are consumed.
            f = sentil.Formula(sentil_mex('formula_predicate', lhs.consume(), ...
                double(int32(op)), rhs.consume()));
        end

        function f = probability(op, threshold, child)
            %PROBABILITY A probabilistic operator P op threshold (child). Consumes child.
            f = sentil.Formula(sentil_mex('formula_probabilistic', double(int32(op)), ...
                double(threshold), child.consume()));
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

        function e = check_rare_event_gpu(obj, model, config)
            %CHECK_RARE_EVENT_GPU Estimate a P >= p (always[0, b] psi) formula on the GPU.
            obj.assertOpen();
            if nargin < 3, config = sentil.RareEventConfig; end
            e = sentil_mex('formula_check_rare_event_gpu', obj.Handle, model.Handle, ...
                config.particles, config.margin, config.seed);
        end

        function f = negate(obj)
            %NEGATE Logical negation. Consumes this formula.
            f = sentil.Formula(sentil_mex('formula_not', obj.consume()));
        end

        function f = conjunction(obj, other)
            %CONJUNCTION This formula and another. Both are consumed.
            f = sentil.Formula(sentil_mex('formula_and', obj.consume(), other.consume()));
        end

        function f = disjunction(obj, other)
            %DISJUNCTION This formula or another. Both are consumed.
            f = sentil.Formula(sentil_mex('formula_or', obj.consume(), other.consume()));
        end

        function f = implies(obj, other)
            %IMPLIES This formula implies another. Both are consumed.
            f = sentil.Formula(sentil_mex('formula_implies', obj.consume(), other.consume()));
        end

        function f = next(obj)
            %NEXT The next-step operator over this formula. Consumes it.
            f = sentil.Formula(sentil_mex('formula_next', obj.consume()));
        end

        function f = always(obj, lower, upper)
            %ALWAYS The always operator, over [lower, upper] if given.
            f = obj.temporal('formula_always', nargin, lower, upper);
        end

        function f = eventually(obj, lower, upper)
            %EVENTUALLY The eventually operator, over [lower, upper] if given.
            f = obj.temporal('formula_eventually', nargin, lower, upper);
        end

        function f = historically(obj, lower, upper)
            %HISTORICALLY The past-time historically operator.
            f = obj.temporal('formula_historically', nargin, lower, upper);
        end

        function f = once(obj, lower, upper)
            %ONCE The past-time once operator.
            f = obj.temporal('formula_once', nargin, lower, upper);
        end

        function f = until(obj, other, lower, upper)
            %UNTIL This formula until another, over [lower, upper] if given.
            if nargin < 3
                f = sentil.Formula(sentil_mex('formula_until', obj.consume(), other.consume(), ...
                    0, 0, 0));
            else
                f = sentil.Formula(sentil_mex('formula_until', obj.consume(), other.consume(), ...
                    double(lower), double(upper), 1));
            end
        end

        function f = since(obj, other, lower, upper)
            %SINCE This formula since another, over [lower, upper] if given.
            if nargin < 3
                f = sentil.Formula(sentil_mex('formula_since', obj.consume(), other.consume(), ...
                    0, 0, 0));
            else
                f = sentil.Formula(sentil_mex('formula_since', obj.consume(), other.consume(), ...
                    double(lower), double(upper), 1));
            end
        end
    end

    methods (Access = private)
        function f = temporal(obj, cmd, argCount, lower, upper)
            if argCount < 3
                f = sentil.Formula(sentil_mex(cmd, obj.consume(), 0, 0, 0));
            else
                f = sentil.Formula(sentil_mex(cmd, obj.consume(), double(lower), double(upper), 1));
            end
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