classdef RingBuffer < handle
    %RINGBUFFER A fixed-capacity rolling window of timed samples with running statistics.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = RingBuffer(capacity)
            obj.Handle = sentil_mex('ring_buffer_create', double(capacity));
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('ring_buffer_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function evicted = push(obj, time, value)
            %PUSH Append a timed sample, returning any evicted sample.
            obj.assertOpen();
            evicted = sentil_mex('ring_buffer_push', obj.Handle, double(time), double(value));
        end

        function clear(obj)
            %CLEAR Drop every sample.
            obj.assertOpen();
            sentil_mex('ring_buffer_clear', obj.Handle);
        end

        function n = length(obj)
            %LENGTH The number of samples held.
            obj.assertOpen();
            n = sentil_mex('ring_buffer_len', obj.Handle);
        end

        function c = capacity(obj)
            %CAPACITY The maximum number of samples.
            obj.assertOpen();
            c = sentil_mex('ring_buffer_capacity', obj.Handle);
        end

        function tf = is_empty(obj)
            obj.assertOpen();
            tf = sentil_mex('ring_buffer_is_empty', obj.Handle);
        end

        function tf = is_full(obj)
            obj.assertOpen();
            tf = sentil_mex('ring_buffer_is_full', obj.Handle);
        end

        function s = front(obj)
            %FRONT The oldest sample, or [].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_front', obj.Handle);
        end

        function s = back(obj)
            %BACK The newest sample, or [].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_back', obj.Handle);
        end

        function s = get(obj, index)
            %GET The 1-based i-th sample from the front, or [].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_get', obj.Handle, double(index));
        end

        function s = pop_front(obj)
            %POP_FRONT Remove and return the oldest sample, or [].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_pop_front', obj.Handle);
        end

        function s = pop_back(obj)
            %POP_BACK Remove and return the newest sample, or [].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_pop_back', obj.Handle);
        end

        function s = closest_to_time(obj, time)
            %CLOSEST_TO_TIME The held sample nearest a time, or [].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_closest_to_time', obj.Handle, double(time));
        end

        function m = mean(obj)
            %MEAN The mean of the held values, or [] when empty.
            obj.assertOpen();
            m = sentil_mex('ring_buffer_mean', obj.Handle);
        end

        function v = variance(obj)
            obj.assertOpen();
            v = sentil_mex('ring_buffer_variance', obj.Handle);
        end

        function s = std_dev(obj)
            obj.assertOpen();
            s = sentil_mex('ring_buffer_std_dev', obj.Handle);
        end

        function m = min(obj)
            obj.assertOpen();
            m = sentil_mex('ring_buffer_min', obj.Handle);
        end

        function m = max(obj)
            obj.assertOpen();
            m = sentil_mex('ring_buffer_max', obj.Handle);
        end

        function recompute_statistics(obj)
            %RECOMPUTE_STATISTICS Recompute the running statistics from scratch.
            obj.assertOpen();
            sentil_mex('ring_buffer_recompute_statistics', obj.Handle);
        end

        function v = at_time(obj, time)
            %AT_TIME The value at an exact held time, or [].
            obj.assertOpen();
            v = sentil_mex('ring_buffer_at_time', obj.Handle, double(time));
        end

        function r = time_range(obj)
            %TIME_RANGE The [start, end] times spanned, or [] when empty.
            obj.assertOpen();
            r = sentil_mex('ring_buffer_time_range', obj.Handle);
        end

        function s = between(obj, startTime, endTime)
            %BETWEEN The samples with times in [start, end].
            obj.assertOpen();
            s = sentil_mex('ring_buffer_between', obj.Handle, double(startTime), double(endTime));
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this ring buffer has been closed');
            end
        end
    end
end