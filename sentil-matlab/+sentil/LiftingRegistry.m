classdef LiftingRegistry < handle
    %LIFTINGREGISTRY Per-variable noise models used to lift a clean trace.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = LiftingRegistry(handle)
            if nargin == 1 && isa(handle, 'uint64') && isscalar(handle)
                obj.Handle = handle;
            else
                obj.Handle = sentil_mex('lifting_create');
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('lifting_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function register(obj, variable, model, interaction)
            %REGISTER Attach a noise model to a variable. The model is consumed.
            obj.assertOpen();
            if nargin < 4
                interaction = sentil.NoiseInteraction.Additive;
            end
            registry = obj.Handle;
            sentil_mex('lifting_register', registry, char(variable), model.consume(), ...
                double(int32(interaction)));
        end

        function v = variables(obj)
            %VARIABLES The variables that carry a noise model, sorted.
            obj.assertOpen();
            v = sentil_mex('lifting_variables', obj.Handle);
        end

        function tf = is_empty(obj)
            %IS_EMPTY Whether no variable carries a noise model.
            obj.assertOpen();
            tf = sentil_mex('lifting_is_empty', obj.Handle);
        end

        function t = lift(obj, trace, seed)
            %LIFT One seeded noisy realization of the trace.
            obj.assertOpen();
            t = sentil.Trace(uint64(sentil_mex('lifting_lift', obj.Handle, trace.Handle, ...
                double(seed))));
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this registry has been closed');
            end
        end
    end
end