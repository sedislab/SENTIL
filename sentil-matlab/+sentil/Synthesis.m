classdef Synthesis
    %SYNTHESIS Open-loop trajectory synthesis and the smooth-robustness helpers.

    methods (Static)
        function r = synthesize(model, spec, bounds, backend, maxIters, population, smooth)
            %SYNTHESIZE Find an input sequence for the model that best satisfies the spec.
            if nargin < 3 || isempty(bounds)
                boundsHandle = uint64(0);
            else
                boundsHandle = bounds.Handle;
            end
            if nargin < 4, backend = sentil.Backend.Auto; end
            if nargin < 5, maxIters = 0; end
            if nargin < 6, population = 0; end
            if nargin < 7, smooth = sentil.SmoothConfig; end
            r = sentil_mex('synthesize', model.Handle, spec.Handle, boundsHandle, ...
                smooth.temperature, double(int32(smooth.kind)), double(maxIters), ...
                double(int32(backend)), double(population), model.Box);
            r.backend = sentil.Backend(r.backend);
        end

        function v = soft_min(values, temperature)
            %SOFT_MIN A smooth lower bound on the minimum of the values.
            if nargin < 2, temperature = 10; end
            v = sentil_mex('soft_min', double(values), double(temperature));
        end

        function v = soft_max(values, temperature)
            %SOFT_MAX A smooth upper bound on the maximum of the values.
            if nargin < 2, temperature = 10; end
            v = sentil_mex('soft_max', double(values), double(temperature));
        end

        function [point, value] = maximize(objective, start, bounds, maxIters)
            %MAXIMIZE Climb an objective returning [value, gradient] from start within bounds.
            if nargin < 3 || isempty(bounds)
                bounds = sentil.Bounds.unbounded(numel(start));
            end
            if nargin < 4, maxIters = 0; end
            [point, value] = sentil_mex('maximize', objective, double(start), bounds.Handle, ...
                double(maxIters));
        end

        function [point, value] = cma_es(objective, start, bounds, config)
            %CMA_ES Maximize a scalar objective with gradient-free CMA-ES.
            if nargin < 3 || isempty(bounds)
                bounds = sentil.Bounds.unbounded(numel(start));
            end
            if nargin < 4, config = sentil.CmaConfig; end
            [point, value] = sentil_mex('cma_es', objective, double(start), bounds.Handle, ...
                config.population, config.max_generations, config.initial_step, config.tol_step, ...
                config.seed);
        end

        function [point, value] = cma_es_batched(objective, start, bounds, config)
            %CMA_ES_BATCHED CMA-ES with an objective that scores a whole population at once.
            if nargin < 3 || isempty(bounds)
                bounds = sentil.Bounds.unbounded(numel(start));
            end
            if nargin < 4, config = sentil.CmaConfig; end
            [point, value] = sentil_mex('cma_es_batched', objective, double(start), ...
                bounds.Handle, config.population, config.max_generations, config.initial_step, ...
                config.tol_step, config.seed);
        end

        function p = mine_tightest_parameter(make, traces, lower, upper)
            %MINE_TIGHTEST_PARAMETER The tightest parameter in [lower, upper] for which make(param) holds on every trace.
            handles = zeros(1, numel(traces), 'uint64');
            for i = 1:numel(traces)
                handles(i) = traces(i).Handle;
            end
            p = sentil_mex('mine_tightest_parameter', make, handles, double(lower), double(upper));
        end
    end
end