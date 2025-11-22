classdef CmaConfig
    %CMACONFIG Settings for CMA-ES search.

    properties
        population (1, 1) double = 0
        max_generations (1, 1) double = 300
        initial_step (1, 1) double = 0.3
        tol_step (1, 1) double = 1e-11
        seed (1, 1) double = 42
    end
end