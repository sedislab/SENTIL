classdef SimExpr < handle
    %SIMEXPR A term in a declarative stochastic model's update rule.

    properties (SetAccess = private, Hidden)
        Handle uint64 = uint64(0)
    end

    methods (Static)
        function e = prev(variable)
            %PREV The previous value of a variable, by its 1-based position.
            e = sentil.SimExpr(sentil_mex('sim_expr_prev', double(variable)));
        end
        function e = time()
            %TIME The current time.
            e = sentil.SimExpr(sentil_mex('sim_expr_time'));
        end
        function e = constant(value)
            %CONSTANT A constant value.
            e = sentil.SimExpr(sentil_mex('sim_expr_const', double(value)));
        end
        function e = noise(source)
            %NOISE A draw from a noise source, by its 1-based position.
            e = sentil.SimExpr(sentil_mex('sim_expr_noise', double(source)));
        end
        function e = call(name, args)
            %CALL A named function of the argument expressions. Consumes them.
            e = sentil.SimExpr(sentil_mex('sim_expr_call', char(name), consume_handles(args)));
        end
    end

    methods
        function obj = SimExpr(handle)
            obj.Handle = handle;
        end

        function delete(obj)
            if obj.Handle ~= 0
                sentil_mex('sim_expr_destroy', obj.Handle);
                obj.Handle = uint64(0);
            end
        end

        function e = add(obj, other)
            %ADD This plus another expression. Both are consumed.
            e = sentil.SimExpr(sentil_mex('sim_expr_add', obj.consume(), other.consume()));
        end
        function e = sub(obj, other)
            %SUB This minus another expression. Both are consumed.
            e = sentil.SimExpr(sentil_mex('sim_expr_sub', obj.consume(), other.consume()));
        end
        function e = mul(obj, other)
            %MUL This times another expression. Both are consumed.
            e = sentil.SimExpr(sentil_mex('sim_expr_mul', obj.consume(), other.consume()));
        end
        function e = div(obj, other)
            %DIV This divided by another expression. Both are consumed.
            e = sentil.SimExpr(sentil_mex('sim_expr_div', obj.consume(), other.consume()));
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