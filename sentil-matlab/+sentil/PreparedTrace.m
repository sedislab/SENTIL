classdef PreparedTrace < handle
    %PREPAREDTRACE A trace with precomputed interpolation, ready to resample.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = PreparedTrace(handle)
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('prepared_trace_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function t = resample(obj, times)
            %RESAMPLE The trace read onto a new time grid.
            obj.assertOpen();
            t = sentil.Trace(uint64(sentil_mex('prepared_trace_resample', obj.Handle, ...
                double(times))));
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this prepared trace has been closed');
            end
        end
    end
end