package io.github.sedislab.sentil;

/** A binary arithmetic operator inside an expression. */
public enum BinaryOp {
    ADD(0),
    SUB(1),
    MUL(2),
    DIV(3),
    MOD(4),
    POW(5);

    private final int code;

    BinaryOp(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}