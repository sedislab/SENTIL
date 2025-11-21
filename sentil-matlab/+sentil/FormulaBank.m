classdef FormulaBank < handle
    %FORMULABANK A batch of named formulas evaluated together over one trace.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods
        function obj = FormulaBank()
            obj.Handle = sentil_mex('formula_bank_create');
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('formula_bank_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function add(obj, id, spec)
            %ADD Register a formula under an id, from a string or a borrowed sentil.Formula.
            obj.assertOpen();
            if isa(spec, 'sentil.Formula')
                sentil_mex('formula_bank_add_formula', obj.Handle, char(id), spec.Handle);
            else
                sentil_mex('formula_bank_add', obj.Handle, char(id), char(spec));
            end
        end

        function v = ids(obj)
            %IDS The registered ids in insertion order.
            obj.assertOpen();
            v = sentil_mex('formula_bank_ids', obj.Handle);
        end

        function n = length(obj)
            %LENGTH The number of formulas.
            obj.assertOpen();
            n = sentil_mex('formula_bank_len', obj.Handle);
        end

        function tf = is_empty(obj)
            %IS_EMPTY Whether no formula is registered.
            obj.assertOpen();
            tf = sentil_mex('formula_bank_is_empty', obj.Handle);
        end

        function m = robustness(obj, trace)
            %ROBUSTNESS The robustness of every formula over the trace, keyed by id.
            obj.assertOpen();
            m = obj.toMap(sentil_mex('formula_bank_robustness', obj.Handle, trace.Handle));
        end

        function m = robustness_dense(obj, trace)
            %ROBUSTNESS_DENSE The dense-time robustness of every formula, keyed by id.
            obj.assertOpen();
            m = obj.toMap(sentil_mex('formula_bank_robustness_dense', obj.Handle, trace.Handle));
        end
    end

    methods (Static, Access = private)
        function m = toMap(result)
            if isempty(result.ids)
                m = containers.Map('KeyType', 'char', 'ValueType', 'double');
            else
                m = containers.Map(result.ids, num2cell(result.values));
            end
        end
    end

    methods (Access = private)
        function assertOpen(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this bank has been closed');
            end
        end
    end
end