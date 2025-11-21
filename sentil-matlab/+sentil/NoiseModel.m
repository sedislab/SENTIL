classdef NoiseModel < handle
    %NOISEMODEL A sensor noise distribution.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function n = dirac(value)
            n = sentil.NoiseModel(sentil_mex('noise_dirac', double(value)));
        end
        function n = gaussian(mean, stdDev)
            n = sentil.NoiseModel(sentil_mex('noise_gaussian', double(mean), double(stdDev)));
        end
        function n = uniform(low, high)
            n = sentil.NoiseModel(sentil_mex('noise_uniform', double(low), double(high)));
        end
        function n = log_normal(mu, sigma)
            n = sentil.NoiseModel(sentil_mex('noise_log_normal', double(mu), double(sigma)));
        end
        function n = exponential(lambda)
            n = sentil.NoiseModel(sentil_mex('noise_exponential', double(lambda)));
        end
        function n = gamma(shape, scale)
            n = sentil.NoiseModel(sentil_mex('noise_gamma', double(shape), double(scale)));
        end
        function n = beta(alpha, betaParam)
            n = sentil.NoiseModel(sentil_mex('noise_beta', double(alpha), double(betaParam)));
        end
        function n = weibull(shape, scale)
            n = sentil.NoiseModel(sentil_mex('noise_weibull', double(shape), double(scale)));
        end
        function n = rayleigh(scale)
            n = sentil.NoiseModel(sentil_mex('noise_rayleigh', double(scale)));
        end
        function n = gumbel(location, scale)
            n = sentil.NoiseModel(sentil_mex('noise_gumbel', double(location), double(scale)));
        end
        function n = cauchy(location, scale)
            n = sentil.NoiseModel(sentil_mex('noise_cauchy', double(location), double(scale)));
        end
        function n = student_t(df, location, scale)
            n = sentil.NoiseModel(sentil_mex('noise_student_t', double(df), double(location), ...
                double(scale)));
        end
        function n = truncated_normal(mean, stdDev, lower, upper)
            n = sentil.NoiseModel(sentil_mex('noise_truncated_normal', double(mean), ...
                double(stdDev), double(lower), double(upper)));
        end
        function n = poisson(lambda)
            n = sentil.NoiseModel(sentil_mex('noise_poisson', double(lambda)));
        end
        function n = binomial(trials, p)
            n = sentil.NoiseModel(sentil_mex('noise_binomial', double(trials), double(p)));
        end
        function n = bootstrap(residuals)
            %BOOTSTRAP An empirical model resampled from residuals.
            n = sentil.NoiseModel(sentil_mex('noise_bootstrap', double(residuals)));
        end
        function n = mixture(weights, models)
            %MIXTURE A weighted mixture. The component models are consumed.
            handles = zeros(1, numel(models), 'uint64');
            for i = 1:numel(models)
                handles(i) = models(i).consume();
            end
            n = sentil.NoiseModel(sentil_mex('noise_mixture', double(weights), handles));
        end
        function n = fit_gaussian(samples)
            n = sentil.NoiseModel(sentil_mex('noise_fit_gaussian', double(samples)));
        end
        function n = fit_bootstrap(samples)
            n = sentil.NoiseModel(sentil_mex('noise_fit_bootstrap', double(samples)));
        end
        function n = fit_bootstrap_reservoir(samples, maxSamples)
            n = sentil.NoiseModel(sentil_mex('noise_fit_bootstrap_reservoir', double(samples), ...
                double(maxSamples)));
        end
        function n = fit_gaussian_mixture(samples, components, maxIters)
            n = sentil.NoiseModel(sentil_mex('noise_fit_gaussian_mixture', double(samples), ...
                double(components), double(maxIters)));
        end
        function n = from_json(json)
            n = sentil.NoiseModel(sentil_mex('noise_from_json', json));
        end
        function n = from_file(path)
            n = sentil.NoiseModel(sentil_mex('noise_from_file', path));
        end

        function r = residuals(groundTruth, sensor, interaction)
            %RESIDUALS The residuals between paired truth and sensor readings.
            if nargin < 3
                interaction = sentil.NoiseInteraction.Additive;
            end
            r = sentil_mex('noise_residuals', double(groundTruth), double(sensor), ...
                double(int32(interaction)));
        end
    end

    methods
        function obj = NoiseModel(handle)
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('noise_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function m = mean(obj)
            %MEAN The analytic mean, or [] when undefined.
            obj.assertOpen();
            m = sentil_mex('noise_mean', obj.Handle);
        end

        function v = variance(obj)
            %VARIANCE The analytic variance, or [] when undefined.
            obj.assertOpen();
            v = sentil_mex('noise_variance', obj.Handle);
        end

        function s = to_json(obj)
            %TO_JSON The model as a JSON string.
            obj.assertOpen();
            s = sentil_mex('noise_to_json', obj.Handle);
        end
    end

    methods (Hidden)
        function h = consume(obj)
            %CONSUME Surrender the native handle, leaving this model closed.
            obj.assertOpen();
            h = obj.Handle;
            obj.Handle = uint64(0);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this noise model has been closed');
            end
        end
    end
end