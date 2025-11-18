package io.github.sedislab.sentil;

/**
 * A term in a declarative {@link SimModel}'s dynamics. Every method consumes the terms
 * it acts on.
 */
public final class SimExpr extends NativeResource {
    private interface Combine {
        long apply(long left, long right) throws SentilException;
    }

    SimExpr(long handle) {
        super(handle, NativeLib::simExprDestroy);
    }

    /** The previous step's value of the variable at this index. */
    public static SimExpr prev(long variable) throws SentilException {
        return new SimExpr(NativeLib.simExprPrev(variable));
    }

    /** The current time. */
    public static SimExpr time() throws SentilException {
        return new SimExpr(NativeLib.simExprTime());
    }

    /** A constant term. */
    public static SimExpr constant(double value) throws SentilException {
        return new SimExpr(NativeLib.simExprConst(value));
    }

    /** A draw from the noise source at this index. */
    public static SimExpr noise(long source) throws SentilException {
        return new SimExpr(NativeLib.simExprNoise(source));
    }

    private SimExpr binary(SimExpr other, Combine op) throws SentilException {
        long left = handle();
        long right = other.handle();
        disown();
        other.disown();
        return new SimExpr(op.apply(left, right));
    }

    private SimExpr call(String name, SimExpr... operands) throws SentilException {
        long[] handles = new long[operands.length];
        for (int i = 0; i < operands.length; i++) {
            handles[i] = operands[i].handle();
        }
        for (SimExpr operand : operands) {
            operand.disown();
        }
        return new SimExpr(NativeLib.simExprCall(name, handles));
    }

    public SimExpr add(SimExpr other) throws SentilException {
        return binary(other, NativeLib::simExprAdd);
    }

    public SimExpr add(double other) throws SentilException {
        return binary(constant(other), NativeLib::simExprAdd);
    }

    public SimExpr sub(SimExpr other) throws SentilException {
        return binary(other, NativeLib::simExprSub);
    }

    public SimExpr sub(double other) throws SentilException {
        return binary(constant(other), NativeLib::simExprSub);
    }

    public SimExpr mul(SimExpr other) throws SentilException {
        return binary(other, NativeLib::simExprMul);
    }

    public SimExpr mul(double other) throws SentilException {
        return binary(constant(other), NativeLib::simExprMul);
    }

    public SimExpr div(SimExpr other) throws SentilException {
        return binary(other, NativeLib::simExprDiv);
    }

    public SimExpr div(double other) throws SentilException {
        return binary(constant(other), NativeLib::simExprDiv);
    }

    public SimExpr min(SimExpr other) throws SentilException {
        return call("min", this, other);
    }

    public SimExpr max(SimExpr other) throws SentilException {
        return call("max", this, other);
    }

    public SimExpr abs() throws SentilException {
        return call("abs", this);
    }

    public SimExpr sin() throws SentilException {
        return call("sin", this);
    }

    public SimExpr cos() throws SentilException {
        return call("cos", this);
    }

    public SimExpr sqrt() throws SentilException {
        return call("sqrt", this);
    }

    public SimExpr exp() throws SentilException {
        return call("exp", this);
    }

    public SimExpr ln() throws SentilException {
        return call("ln", this);
    }

    public SimExpr negate() throws SentilException {
        return constant(0.0).sub(this);
    }
}