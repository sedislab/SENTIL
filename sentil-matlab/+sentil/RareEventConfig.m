classdef RareEventConfig
    %RAREEVENTCONFIG Settings for adaptive multilevel splitting.

    properties
        particles (1, 1) double = 4096
        margin (1, 1) double = 0
        seed (1, 1) double = 42
    end
end