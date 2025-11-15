package io.github.sedislab.sentil;

/**
 * An arithmetic term inside a predicate. Every method consumes the terms it acts on,
 * as in {@code Expr.var("x").mul(2).gt(5)}.
 */
public final class Expr extends NativeResource {
    Expr(long handle) {
        super(handle, NativeLib::exprDestroy);
    }

    /** A term that reads the named variable. */
    public static Expr var(String name) throws SentilException {
        return new Expr(NativeLib.exprVariable(name));
    }

    /** A constant term. */
    public static Expr constant(double value) throws SentilException {
        return new Expr(NativeLib.exprLiteral(value));
    }

    private Expr binary(BinaryOp op, Expr other) throws SentilException {
        long left = handle();
        long right = other.handle();
        disown();
        other.disown();
        return new Expr(NativeLib.exprBinary(op.code(), left, right));
    }

    private Expr call(String name, Expr... operands) throws SentilException {
        long[] handles = new long[operands.length];
        for (int i = 0; i < operands.length; i++) {
            handles[i] = operands[i].handle();
        }
        for (Expr operand : operands) {
            operand.disown();
        }
        return new Expr(NativeLib.exprCall(name, handles));
    }

    public Expr add(Expr other) throws SentilException {
        return binary(BinaryOp.ADD, other);
    }

    public Expr add(double other) throws SentilException {
        return binary(BinaryOp.ADD, constant(other));
    }

    public Expr sub(Expr other) throws SentilException {
        return binary(BinaryOp.SUB, other);
    }

    public Expr sub(double other) throws SentilException {
        return binary(BinaryOp.SUB, constant(other));
    }

    public Expr mul(Expr other) throws SentilException {
        return binary(BinaryOp.MUL, other);
    }

    public Expr mul(double other) throws SentilException {
        return binary(BinaryOp.MUL, constant(other));
    }

    public Expr div(Expr other) throws SentilException {
        return binary(BinaryOp.DIV, other);
    }

    public Expr div(double other) throws SentilException {
        return binary(BinaryOp.DIV, constant(other));
    }

    public Expr mod(Expr other) throws SentilException {
        return binary(BinaryOp.MOD, other);
    }

    public Expr mod(double other) throws SentilException {
        return binary(BinaryOp.MOD, constant(other));
    }

    public Expr pow(Expr exponent) throws SentilException {
        return binary(BinaryOp.POW, exponent);
    }

    public Expr pow(double exponent) throws SentilException {
        return binary(BinaryOp.POW, constant(exponent));
    }

    public Expr min(Expr other) throws SentilException {
        return call("min", this, other);
    }

    public Expr min(double other) throws SentilException {
        return call("min", this, constant(other));
    }

    public Expr max(Expr other) throws SentilException {
        return call("max", this, other);
    }

    public Expr max(double other) throws SentilException {
        return call("max", this, constant(other));
    }

    public Expr abs() throws SentilException {
        return call("abs", this);
    }

    public Expr sin() throws SentilException {
        return call("sin", this);
    }

    public Expr cos() throws SentilException {
        return call("cos", this);
    }

    public Expr tan() throws SentilException {
        return call("tan", this);
    }

    public Expr sqrt() throws SentilException {
        return call("sqrt", this);
    }

    public Expr exp() throws SentilException {
        return call("exp", this);
    }

    public Expr log() throws SentilException {
        return call("log", this);
    }

    public Expr ln() throws SentilException {
        return call("ln", this);
    }

    public Expr floor() throws SentilException {
        return call("floor", this);
    }

    public Expr ceil() throws SentilException {
        return call("ceil", this);
    }

    public Expr negate() throws SentilException {
        return constant(0.0).sub(this);
    }

    private Formula compare(ComparisonOp op, Expr other) throws SentilException {
        long left = handle();
        long right = other.handle();
        disown();
        other.disown();
        return new Formula(NativeLib.formulaPredicate(left, op.code(), right));
    }

    public Formula lt(Expr other) throws SentilException {
        return compare(ComparisonOp.LT, other);
    }

    public Formula lt(double other) throws SentilException {
        return compare(ComparisonOp.LT, constant(other));
    }

    public Formula le(Expr other) throws SentilException {
        return compare(ComparisonOp.LE, other);
    }

    public Formula le(double other) throws SentilException {
        return compare(ComparisonOp.LE, constant(other));
    }

    public Formula gt(Expr other) throws SentilException {
        return compare(ComparisonOp.GT, other);
    }

    public Formula gt(double other) throws SentilException {
        return compare(ComparisonOp.GT, constant(other));
    }

    public Formula ge(Expr other) throws SentilException {
        return compare(ComparisonOp.GE, other);
    }

    public Formula ge(double other) throws SentilException {
        return compare(ComparisonOp.GE, constant(other));
    }

    public Formula eq(Expr other) throws SentilException {
        return compare(ComparisonOp.EQ, other);
    }

    public Formula eq(double other) throws SentilException {
        return compare(ComparisonOp.EQ, constant(other));
    }

    public Formula ne(Expr other) throws SentilException {
        return compare(ComparisonOp.NE, other);
    }

    public Formula ne(double other) throws SentilException {
        return compare(ComparisonOp.NE, constant(other));
    }
}