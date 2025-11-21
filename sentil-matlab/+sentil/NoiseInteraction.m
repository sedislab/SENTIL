classdef NoiseInteraction < int32
    %NOISEINTERACTION How sensor noise combines with the ground truth.

    enumeration
        Additive (0)
        Multiplicative (1)
    end
end