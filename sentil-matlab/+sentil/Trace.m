classdef Trace < handle
    %TRACE A multivariate timed signal.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function t = from_csv(text)
            %FROM_CSV Parse a trace from comma-separated text with a header row.
            t = sentil.Trace(sentil_mex('trace_from_csv', text));
        end

        function t = from_tsv(text)
            %FROM_TSV Parse a trace from tab-separated text with a header row.
            t = sentil.Trace(sentil_mex('trace_from_tsv', text));
        end

        function t = from_path(path)
            %FROM_PATH Load a trace from a CSV or TSV file, chosen by extension.
            t = sentil.Trace(sentil_mex('trace_from_path', path));
        end
    end

    methods
        function obj = Trace(varargin)
            %TRACE Build a trace from a time vector, optionally with one named signal.
            if nargin == 1 && isa(varargin{1}, 'uint64') && isscalar(varargin{1})
                obj.Handle = varargin{1};
            elseif nargin == 1
                obj.Handle = sentil_mex('trace_create', double(varargin{1}));
            elseif nargin == 3
                obj.Handle = sentil_mex('trace_from_signal', double(varargin{1}), ...
                    char(varargin{2}), double(varargin{3}));
            else
                error('sentil:trace', ...
                    'use sentil.Trace(times) or sentil.Trace(times, name, values)');
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('trace_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function add_signal(obj, name, values)
            %ADD_SIGNAL Attach a named signal sampled on the trace's time grid.
            obj.assertOpen();
            sentil_mex('trace_add_signal', obj.Handle, char(name), double(values));
        end

        function n = length(obj)
            %LENGTH The number of samples.
            obj.assertOpen();
            n = sentil_mex('trace_len', obj.Handle);
        end

        function tf = is_empty(obj)
            %IS_EMPTY Whether the trace has no samples.
            obj.assertOpen();
            tf = sentil_mex('trace_is_empty', obj.Handle);
        end

        function t = times(obj)
            %TIMES The time grid as a row vector.
            obj.assertOpen();
            t = sentil_mex('trace_times', obj.Handle);
        end

        function v = variables(obj)
            %VARIABLES The signal names.
            obj.assertOpen();
            v = sentil_mex('trace_variables', obj.Handle);
        end

        function s = signal(obj, name)
            %SIGNAL The named signal, or [] if the trace has no such name.
            obj.assertOpen();
            s = sentil_mex('trace_signal', obj.Handle, char(name));
        end

        function t = resample(obj, newTimes, interp)
            %RESAMPLE A copy of the trace read onto a new time grid.
            obj.assertOpen();
            if nargin < 3
                interp = sentil.Interpolation.Linear;
            end
            t = sentil.Trace(sentil_mex('trace_resample', obj.Handle, double(newTimes), ...
                double(int32(interp))));
        end

        function p = prepare(obj, interp)
            %PREPARE A sentil.PreparedTrace with precomputed interpolation.
            obj.assertOpen();
            if nargin < 2
                interp = sentil.Interpolation.Linear;
            end
            p = sentil.PreparedTrace(uint64(sentil_mex('trace_prepare', obj.Handle, ...
                double(int32(interp)))));
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this trace has been closed');
            end
        end
    end
end