classdef MultiMonitor < handle
    %MULTIMONITOR Several named formulas advanced together on one clock.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = MultiMonitor()
            obj.Handle = sentil_mex('multi_monitor_create');
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('multi_monitor_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function add(obj, id, spec)
            %ADD Register a formula under an id, from a string or a borrowed sentil.Formula.
            obj.assertOpen();
            if isa(spec, 'sentil.Formula')
                sentil_mex('multi_monitor_add_formula', obj.Handle, char(id), spec.Handle);
            else
                sentil_mex('multi_monitor_add', obj.Handle, char(id), char(spec));
            end
        end

        function add_probabilistic(obj, id, formula, lifting, config)
            %ADD_PROBABILISTIC Register a P-wrapped formula tracked with a lifted ensemble.
            obj.assertOpen();
            if nargin < 5, config = sentil.SmcConfig; end
            sentil_mex('multi_monitor_add_probabilistic', obj.Handle, char(id), formula.Handle, ...
                lifting.Handle, config.samples, config.confidence, config.seed, ...
                double(int32(config.method)));
        end

        function tf = remove(obj, id)
            %REMOVE Drop a monitor by id.
            obj.assertOpen();
            tf = sentil_mex('multi_monitor_remove', obj.Handle, char(id));
        end

        function reset(obj)
            %RESET Clear every monitor's streaming state.
            obj.assertOpen();
            sentil_mex('multi_monitor_reset', obj.Handle);
        end

        function n = length(obj)
            %LENGTH The number of registered monitors.
            obj.assertOpen();
            n = sentil_mex('multi_monitor_len', obj.Handle);
        end

        function tf = is_empty(obj)
            %IS_EMPTY Whether no monitor is registered.
            obj.assertOpen();
            tf = sentil_mex('multi_monitor_is_empty', obj.Handle);
        end

        function v = ids(obj)
            %IDS The registered ids in insertion order.
            obj.assertOpen();
            v = sentil_mex('multi_monitor_ids', obj.Handle);
        end

        function m = update(obj, time, sample)
            %UPDATE Advance every monitor at one sample, returning verdicts keyed by id.
            obj.assertOpen();
            [names, values] = unpack_sample(sample);
            arr = sentil_mex('multi_monitor_update', obj.Handle, double(time), names, values);
            m = containers.Map('KeyType', 'char', 'ValueType', 'any');
            for i = 1:numel(arr)
                e = arr(i);
                m(e.id) = struct('resolved', e.resolved, 'satisfied', e.satisfied, ...
                    'value', e.value, 'lower', e.lower, 'upper', e.upper);
            end
        end

        function p = probability(obj, id)
            %PROBABILITY The last satisfaction probability for the formula under id.
            obj.assertOpen();
            p = sentil_mex('multi_monitor_probability', obj.Handle, char(id));
        end

        function m = probabilities(obj)
            %PROBABILITIES The last probability of every monitor, keyed by id.
            obj.assertOpen();
            names = obj.ids();
            m = containers.Map('KeyType', 'char', 'ValueType', 'double');
            for i = 1:numel(names)
                m(names{i}) = obj.probability(names{i});
            end
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this monitor has been closed');
            end
        end
    end
end