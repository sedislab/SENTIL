classdef OnlineMonitor < handle
    %ONLINEMONITOR A streaming monitor with O(1) amortized cost per sample.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function m = from_formula(formula)
            %FROM_FORMULA Build from a borrowed sentil.Formula.
            m = sentil.OnlineMonitor(uint64(sentil_mex('stream_monitor_from_formula', ...
                formula.Handle)));
        end

        function m = with_lifting(formula, lifting, config)
            %WITH_LIFTING A probabilistic streaming monitor over a lifted particle ensemble.
            if nargin < 3, config = sentil.SmcConfig; end
            m = sentil.OnlineMonitor(uint64(sentil_mex('stream_monitor_with_lifting', ...
                formula.Handle, lifting.Handle, config.samples, config.confidence, ...
                config.seed, double(int32(config.method)))));
        end
    end

    methods
        function obj = OnlineMonitor(spec)
            %ONLINEMONITOR Build from a formula string.
            if isa(spec, 'uint64') && isscalar(spec)
                obj.Handle = spec;
            elseif ischar(spec) || isstring(spec)
                obj.Handle = sentil_mex('stream_monitor_create', char(spec));
            else
                error('sentil:monitor', 'OnlineMonitor(formulaString)');
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('stream_monitor_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function n = variable_count(obj)
            %VARIABLE_COUNT The number of variables the formula reads.
            obj.assertOpen();
            n = sentil_mex('stream_monitor_variable_count', obj.Handle);
        end

        function idx = symbol_index(obj, name)
            %SYMBOL_INDEX The 1-based position of a variable in update_packed order.
            obj.assertOpen();
            idx = sentil_mex('stream_monitor_symbol_index', obj.Handle, char(name));
        end

        function r = update(obj, time, sample)
            %UPDATE Fold one sample, given as a struct or containers.Map.
            obj.assertOpen();
            [names, values] = unpack_sample(sample);
            r = sentil_mex('stream_monitor_update', obj.Handle, double(time), names, values);
        end

        function r = update_packed(obj, time, values)
            %UPDATE_PACKED Fold one sample with values already in symbol_index order.
            obj.assertOpen();
            r = sentil_mex('stream_monitor_update_packed', obj.Handle, double(time), ...
                double(values));
        end

        function r = run(obj, trace)
            %RUN Replay a whole trace, returning the per-sample verdicts.
            obj.assertOpen();
            r = sentil_mex('stream_monitor_run', obj.Handle, trace.Handle);
        end

        function reset(obj)
            %RESET Clear the streaming state.
            obj.assertOpen();
            sentil_mex('stream_monitor_reset', obj.Handle);
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