classdef Controller < handle
    %CONTROLLER A receding-horizon controller with a per-step wall-clock budget.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
        InputWidth double = 0
        ModelBox uint64 = uint64(0)
    end

    methods
        function obj = Controller(model, spec, inputWidth, budgetNs, bounds, smooth)
            %CONTROLLER Build over a model and spec, both consumed. budgetNs is in nanoseconds.
            if nargin < 5 || isempty(bounds)
                boundsHandle = uint64(0);
            else
                boundsHandle = bounds.Handle;
            end
            if nargin < 6, smooth = sentil.SmoothConfig; end
            % The C ABI consumes the model handle even on a null return.
            [modelHandle, box] = model.releaseToController();
            obj.ModelBox = box;
            obj.Handle = sentil_mex('controller_create', modelHandle, spec.consume(), ...
                double(inputWidth), double(budgetNs), boundsHandle, smooth.temperature, ...
                double(int32(smooth.kind)));
            obj.InputWidth = double(inputWidth);
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('controller_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
            if obj.ModelBox ~= 0
                sentil_mex('free_model_box', obj.ModelBox);
                obj.ModelBox = uint64(0);
            end
        end

        function u = control(obj, state)
            %CONTROL Plan from the current state and return the control input.
            obj.assertOpen();
            u = sentil_mex('controller_control', obj.Handle, double(state), obj.InputWidth, ...
                obj.ModelBox);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this controller has been closed');
            end
        end
    end
end