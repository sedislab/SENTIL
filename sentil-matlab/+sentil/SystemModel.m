classdef SystemModel < handle
    %SYSTEMMODEL A dynamical model the synthesizer drives.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
        Box uint64 = uint64(0)
    end

    methods (Static)
        function m = linear(A, B, x0, variables, dt, horizon)
            %LINEAR A linear time-invariant model x_{t+1} = A x_t + B u_t.
            if ~iscell(variables)
                variables = cellstr(variables);
            end
            n = size(A, 1);
            bCols = size(B, 2);
            m = sentil.SystemModel(sentil_mex('linear_model_create', A.', n, B.', bCols, ...
                double(x0(:).'), variables, double(dt), double(horizon)));
        end

        function m = custom(variables, dt, horizon, initialState, inputDimension, rollout)
            %CUSTOM A model whose rollout(initial, input) returns a variables-by-(horizon+1) matrix.
            if ~iscell(variables)
                variables = cellstr(variables);
            end
            [handle, box] = sentil_mex('system_model_create_custom', rollout, variables, ...
                double(dt), double(horizon), double(initialState(:).'), double(inputDimension));
            m = sentil.SystemModel(handle, box);
        end
    end

    methods
        function obj = SystemModel(handle, box)
            obj.Handle = handle;
            if nargin >= 2
                obj.Box = box;
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('system_model_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
            if obj.Box ~= 0
                sentil_mex('free_model_box', obj.Box);
                obj.Box = uint64(0);
            end
        end

        function d = input_dimension(obj)
            %INPUT_DIMENSION The total length of the input sequence.
            obj.assertOpen();
            d = sentil_mex('system_model_input_dimension', obj.Handle);
        end
    end

    methods (Hidden)
        function [h, box] = releaseToController(obj)
            %RELEASETOCONTROLLER Surrender the model handle and rollout box, leaving this model closed.
            obj.assertOpen();
            h = obj.Handle;
            box = obj.Box;
            obj.Handle = uint64(0);
            obj.Box = uint64(0);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this model has been closed');
            end
        end
    end
end