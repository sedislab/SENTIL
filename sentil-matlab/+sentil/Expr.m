classdef Expr < handle
    %EXPR A predicate term.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function e = var(name)
            %VAR A variable term.
            e = sentil.Expr(sentil_mex('expr_variable', char(name)));
        end
        function e = literal(value)
            %LITERAL A constant term.
            e = sentil.Expr(sentil_mex('expr_literal', double(value)));
        end
        function e = binary(op, left, right)
            %BINARY An arithmetic combination by a sentil.BinaryOp. Consumes the operands.
            e = sentil.Expr(sentil_mex('expr_binary', double(int32(op)), left.consume(), ...
                right.consume()));
        end
        function e = call(name, args)
            %CALL A named function of the argument expressions. Consumes them.
            e = sentil.Expr(sentil_mex('expr_call', char(name), consume_handles(args)));
        end
    end

    methods
        function obj = Expr(handle)
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('expr_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function e = add(obj, other)
            %ADD This plus another expression. Both are consumed.
            e = sentil.Expr.binary(sentil.BinaryOp.Add, obj, other);
        end
        function e = sub(obj, other)
            %SUB This minus another expression. Both are consumed.
            e = sentil.Expr.binary(sentil.BinaryOp.Sub, obj, other);
        end
        function e = mul(obj, other)
            %MUL This times another expression. Both are consumed.
            e = sentil.Expr.binary(sentil.BinaryOp.Mul, obj, other);
        end
        function e = div(obj, other)
            %DIV This divided by another expression. Both are consumed.
            e = sentil.Expr.binary(sentil.BinaryOp.Div, obj, other);
        end
        function e = mod(obj, other)
            %MOD This modulo another expression. Both are consumed.
            e = sentil.Expr.binary(sentil.BinaryOp.Mod, obj, other);
        end
        function e = pow(obj, other)
            %POW This raised to another expression. Both are consumed.
            e = sentil.Expr.binary(sentil.BinaryOp.Pow, obj, other);
        end
    end

    methods (Hidden)
        function h = consume(obj)
            if obj.Handle == 0
                error('sentil:handle', 'this expression has been consumed');
            end
            h = obj.Handle;
            obj.Handle = uint64(0);
        end
    end
end