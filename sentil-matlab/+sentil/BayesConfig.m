classdef BayesConfig
    %BAYESCONFIG Settings for a Bayesian sequential test.

    properties
        threshold (1, 1) double
        bayes_factor (1, 1) double = 100
        max_samples (1, 1) double = 100000
        seed (1, 1) double = 42
    end

    methods
        function obj = BayesConfig(threshold)
            %BAYESCONFIG A config for the asserted probability threshold.
            if nargin >= 1
                obj.threshold = threshold;
            end
        end
    end
end