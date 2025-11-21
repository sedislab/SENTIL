classdef Monitor < handle
    %MONITOR An offline and incremental monitor for one formula.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = Monitor(spec, config)
            %MONITOR Build from a formula string or a consumed sentil.Formula.
            cfg = uint64(0);
            if nargin >= 2 && ~isempty(config)
                cfg = config.Handle;
            end
            if isa(spec, 'sentil.Formula')
                obj.Handle = sentil_mex('monitor_create', spec.consume(), cfg);
            elseif ischar(spec) || isstring(spec)
                obj.Handle = sentil_mex('monitor_parse', char(spec), cfg);
            elseif isa(spec, 'uint64') && isscalar(spec)
                obj.Handle = spec;
            else
                error('sentil:monitor', 'Monitor(formulaOrString, config)');
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('monitor_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function r = robustness(obj, trace)
            %ROBUSTNESS Robustness over the trace.
            obj.assertOpen();
            r = sentil_mex('monitor_robustness', obj.Handle, trace.Handle);
        end

        function r = robustness_signal(obj, trace)
            %ROBUSTNESS_SIGNAL Robustness at every sample, as a row vector.
            obj.assertOpen();
            r = sentil_mex('monitor_robustness_signal', obj.Handle, trace.Handle);
        end

        function v = violations(obj, trace)
            %VIOLATIONS The failing time spans, as [start, end] rows.
            obj.assertOpen();
            v = sentil_mex('monitor_violations', obj.Handle, trace.Handle);
        end

        function idx = symbol_index(obj, name)
            %SYMBOL_INDEX The 1-based position of a variable in update_packed order.
            obj.assertOpen();
            idx = sentil_mex('monitor_symbol_index', obj.Handle, char(name));
        end

        function r = update(obj, time, sample)
            %UPDATE Fold one timestamped sample, given as a struct or containers.Map.
            obj.assertOpen();
            [names, values] = unpack_sample(sample);
            r = sentil_mex('monitor_update', obj.Handle, double(time), names, values);
        end

        function r = update_packed(obj, time, values)
            %UPDATE_PACKED Fold one sample with values already in symbol_index order.
            obj.assertOpen();
            r = sentil_mex('monitor_update_packed', obj.Handle, double(time), double(values));
        end

        function reset(obj)
            %RESET Clear streaming state to run a fresh trace.
            obj.assertOpen();
            sentil_mex('monitor_reset', obj.Handle);
        end

        function f = formula(obj)
            %FORMULA A copy of the monitored formula.
            obj.assertOpen();
            f = sentil.Formula(sentil_mex('monitor_formula', obj.Handle));
        end

        function c = config(obj)
            %CONFIG A copy of the monitor's config.
            obj.assertOpen();
            c = sentil.Config(sentil_mex('monitor_config_of', obj.Handle));
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