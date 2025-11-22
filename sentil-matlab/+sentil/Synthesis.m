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
                double(int32(backend)), double(population));
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
    end
end