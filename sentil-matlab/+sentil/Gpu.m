classdef Gpu
    %GPU The GPU capability check.

    methods (Static)
        function tf = is_available()
            %IS_AVAILABLE Whether a usable GPU device is present.
            tf = sentil_mex('gpu_is_available');
        end
    end
end