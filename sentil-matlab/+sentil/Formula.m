classdef Formula < handle
    %FORMULA A parsed SENTIL temporal-logic formula.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function f = parse(text)
            %PARSE Parse a formula from its textual form.
            f = sentil.Formula(sentil_mex('formula_parse', text));
        end
    end

    methods
        function obj = Formula(handle)
            %FORMULA Wrap an engine handle.
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('formula_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function v = variables(obj)
            %VARIABLES The variable names the formula reads.
            obj.assertOpen();
            v = sentil_mex('formula_variables', obj.Handle);
        end

        function s = to_json(obj)
            %TO_JSON The formula as a JSON string.
            obj.assertOpen();
            s = sentil_mex('formula_to_json', obj.Handle);
        end

        function r = robustness(obj, trace)
            %ROBUSTNESS The robustness margin over a trace.
            obj.assertOpen();
            r = sentil_mex('formula_robustness', obj.Handle, trace.Handle);
        end

        function r = robustness_dense(obj, trace)
            %ROBUSTNESS_DENSE The dense-time robustness margin over a trace.
            obj.assertOpen();
            r = sentil_mex('formula_robustness_dense', obj.Handle, trace.Handle);
        end

        function r = robustness_signal(obj, trace)
            %ROBUSTNESS_SIGNAL The robustness at every sample, as a row vector.
            obj.assertOpen();
            r = sentil_mex('formula_robustness_signal', obj.Handle, trace.Handle);
        end

        function r = robustness_dense_signal(obj, trace)
            %ROBUSTNESS_DENSE_SIGNAL Dense robustness at every sample, as a row vector.
            obj.assertOpen();
            r = sentil_mex('formula_robustness_dense_signal', obj.Handle, trace.Handle);
        end

        function v = violations(obj, trace)
            %VIOLATIONS The time spans where the formula fails, as [start, end] rows.
            obj.assertOpen();
            v = sentil_mex('formula_violations', obj.Handle, trace.Handle);
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this formula has been closed');
            end
        end
    end
end