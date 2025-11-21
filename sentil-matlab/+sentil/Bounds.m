classdef Bounds < handle
    %BOUNDS Per-coordinate box bounds on a synthesis decision vector.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function b = unbounded(dimension)
            %UNBOUNDED Bounds that constrain nothing over the given dimension.
            b = sentil.Bounds(uint64(sentil_mex('bounds_unbounded', double(dimension))));
        end
    end

    methods
        function obj = Bounds(lower, upper)
            %BOUNDS Box bounds from per-coordinate lower and upper limits.
            if nargin == 1 && isa(lower, 'uint64') && isscalar(lower)
                obj.Handle = lower;
            else
                obj.Handle = sentil_mex('bounds_create', double(lower), double(upper));
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('bounds_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function p = clamp(obj, point)
            %CLAMP Project a point into the box.
            obj.assertOpen();
            p = sentil_mex('bounds_clamp', obj.Handle, double(point));
        end

        function d = dimension(obj)
            %DIMENSION The number of coordinates.
            obj.assertOpen();
            d = sentil_mex('bounds_dimension', obj.Handle);
        end

        function l = lower(obj)
            %LOWER The per-coordinate lower limits.
            obj.assertOpen();
            l = sentil_mex('bounds_lower', obj.Handle);
        end

        function u = upper(obj)
            %UPPER The per-coordinate upper limits.
            obj.assertOpen();
            u = sentil_mex('bounds_upper', obj.Handle);
        end
    end

    methods (Hidden)
        function h = consume(obj)
            obj.assertOpen();
            h = obj.Handle;
            obj.Handle = uint64(0);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'these bounds have been closed');
            end
        end
    end
end