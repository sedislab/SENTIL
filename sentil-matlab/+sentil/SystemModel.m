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
                sentil_mex('free_system_box', obj.Box);
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
        function h = consume(obj)
            obj.assertOpen();
            h = obj.Handle;
            obj.Handle = uint64(0);
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