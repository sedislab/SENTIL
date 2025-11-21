classdef Stats
    %STATS Binomial confidence intervals and a priori sample sizes.

    methods (Static)
        function ci = wilson(successes, trials, level)
            %WILSON The Wilson score interval.
            if nargin < 3, level = 0.95; end
            ci = sentil_mex('stats_wilson', double(successes), double(trials), double(level));
        end

        function ci = clopper_pearson(successes, trials, level)
            %CLOPPER_PEARSON The exact, conservative interval.
            if nargin < 3, level = 0.95; end
            ci = sentil_mex('stats_clopper_pearson', double(successes), double(trials), ...
                double(level));
        end

        function ci = jeffreys(successes, trials, level)
            %JEFFREYS The Jeffreys-prior interval.
            if nargin < 3, level = 0.95; end
            ci = sentil_mex('stats_jeffreys', double(successes), double(trials), double(level));
        end

        function ci = agresti_coull(successes, trials, level)
            %AGRESTI_COULL The adjusted Wald interval.
            if nargin < 3, level = 0.95; end
            ci = sentil_mex('stats_agresti_coull', double(successes), double(trials), ...
                double(level));
        end

        function ci = interval(method, successes, trials, level)
            %INTERVAL A confidence interval by a chosen sentil.IntervalMethod.
            if nargin < 4, level = 0.95; end
            ci = sentil_mex('stats_interval', double(int32(method)), double(successes), ...
                double(trials), double(level));
        end

        function z = z_score(level)
            %Z_SCORE The two-sided z critical value for a confidence level.
            z = sentil_mex('stats_z_score', double(level));
        end

        function n = chernoff_hoeffding_samples(epsilon, delta)
            %CHERNOFF_HOEFFDING_SAMPLES Samples for a target error and confidence.
            n = sentil_mex('stats_chernoff_hoeffding', double(epsilon), double(delta));
        end

        function n = wilson_samples(epsilon, level)
            %WILSON_SAMPLES Samples for a target Wilson half-width at a confidence level.
            n = sentil_mex('stats_wilson_samples', double(epsilon), double(level));
        end
    end
end