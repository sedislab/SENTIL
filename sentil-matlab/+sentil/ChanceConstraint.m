classdef ChanceConstraint < handle
    %CHANCECONSTRAINT A risk constraint that a spec holds with at least a target probability.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = ChanceConstraint(spec, probability, confidence, tightening)
            %CHANCECONSTRAINT The spec (consumed) must hold with at least probability.
            if nargin < 3, confidence = 0.0; end
            if nargin < 4, tightening = 0.0; end
            obj.Handle = sentil_mex('chance_constraint_create', spec.consume(), ...
                double(probability), double(confidence), double(tightening));
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('chance_constraint_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function r = validate(obj, system, samples, seed)
            %VALIDATE Estimate the satisfaction probability over a sentil.StochasticSystem.
            obj.assertOpen();
            if nargin < 3, samples = 1000; end
            if nargin < 4, seed = 42; end
            r = sentil_mex('chance_constraint_validate', obj.Handle, system.Handle, system.Box, ...
                double(samples), double(seed));
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this constraint has been closed');
            end
        end
    end
end