classdef SafetyFilter < handle
    %SAFETYFILTER A least-restrictive shield over a nominal control input.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = SafetyFilter(bounds)
            %SAFETYFILTER A filter over the given bounds, which are consumed.
            obj.Handle = sentil_mex('safety_filter_create', bounds.consume());
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('safety_filter_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function u = filter(obj, nominal, barrierA, barrierB)
            %FILTER The input closest to nominal satisfying the bounds and each barrier a_i . u >= b_i.
            obj.assertOpen();
            if nargin < 3
                u = sentil_mex('safety_filter_filter', obj.Handle, double(nominal), [], []);
            else
                u = sentil_mex('safety_filter_filter', obj.Handle, double(nominal), ...
                    double(barrierA).', double(barrierB));
            end
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this filter has been closed');
            end
        end
    end
end