classdef StochasticSystem < handle
    %STOCHASTICSYSTEM A sampling-ready stochastic system for rare-event estimation.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
        Box uint64 = uint64(0)
    end

    methods (Static)
        function s = custom(variables, dt, horizon, init, step)
            %CUSTOM A system driven by init(seed) and step(prev, time, seed).
            if ~iscell(variables)
                variables = cellstr(variables);
            end
            [handle, box] = sentil_mex('stochastic_system_create_custom', init, step, ...
                variables, double(dt), double(horizon));
            s = sentil.StochasticSystem(handle, box);
        end
    end

    methods
        function obj = StochasticSystem(handle, box)
            obj.Handle = handle;
            if nargin >= 2
                obj.Box = box;
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('stochastic_system_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
            if obj.Box ~= 0
                sentil_mex('free_system_box', obj.Box);
                obj.Box = uint64(0);
            end
        end

        function t = simulate(obj, seed)
            %SIMULATE One full-horizon trajectory from a seed.
            obj.assertOpen();
            t = sentil.Trace(uint64(sentil_mex('stochastic_system_simulate', obj.Handle, ...
                obj.Box, double(seed))));
        end

        function v = variables(obj)
            obj.assertOpen();
            v = sentil_mex('stochastic_system_variables', obj.Handle);
        end

        function d = dt(obj)
            obj.assertOpen();
            d = sentil_mex('stochastic_system_dt', obj.Handle);
        end

        function h = horizon(obj)
            obj.assertOpen();
            h = sentil_mex('stochastic_system_horizon', obj.Handle);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this system has been closed');
            end
        end
    end
end