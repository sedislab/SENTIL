classdef Config < handle
    %CONFIG Monitor settings.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = Config(mode)
            %CONFIG A config, optionally with a time mode.
            if nargin == 1 && isa(mode, 'uint64') && isscalar(mode)
                obj.Handle = mode;
                return
            end
            obj.Handle = sentil_mex('monitor_config_create');
            if nargin == 1
                obj.set_time(mode);
            end
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('monitor_config_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function obj = set_time(obj, mode)
            %SET_TIME Set the time mode.
            obj.assertOpen();
            sentil_mex('monitor_config_set_time', obj.Handle, double(int32(mode)));
        end

        function m = time_mode(obj)
            %TIME_MODE The configured time mode.
            obj.assertOpen();
            m = sentil.TimeMode(sentil_mex('monitor_config_time_mode', obj.Handle));
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this config has been closed');
            end
        end
    end
end