classdef SimModel < handle
    %SIMMODEL A declarative stochastic model with one init and advance rule per variable.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function m = create(variables, dt, horizon, init, advance, noise)
            %CREATE A model over the named variables. The expressions and noise models are consumed.
            if ~iscell(variables)
                variables = cellstr(variables);
            end
            m = sentil.SimModel(sentil_mex('sim_model_create', variables, double(dt), ...
                double(horizon), consume_handles(init), consume_handles(advance), ...
                consume_handles(noise)));
        end
    end

    methods
        function obj = SimModel(handle)
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('sim_model_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function t = simulate(obj, seed)
            %SIMULATE One full-horizon trajectory from a seed.
            obj.assertOpen();
            t = sentil.Trace(uint64(sentil_mex('sim_model_simulate', obj.Handle, double(seed))));
        end

        function v = variables(obj)
            %VARIABLES The model's variable names.
            obj.assertOpen();
            v = sentil_mex('sim_model_variables', obj.Handle);
        end

        function d = dt(obj)
            %DT The time step.
            obj.assertOpen();
            d = sentil_mex('sim_model_dt', obj.Handle);
        end

        function h = horizon(obj)
            %HORIZON The trajectory length, in steps.
            obj.assertOpen();
            h = sentil_mex('sim_model_horizon', obj.Handle);
        end

        function s = to_stochastic_system(obj)
            %TO_STOCHASTIC_SYSTEM A sampling-ready system for rare-event estimation.
            obj.assertOpen();
            s = sentil.StochasticSystem(uint64(sentil_mex('sim_model_to_stochastic_system', ...
                obj.Handle)));
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