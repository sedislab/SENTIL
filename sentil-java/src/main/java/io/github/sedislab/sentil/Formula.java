package io.github.sedislab.sentil;

/** A parsed PrSTL formula. The combinators consume their operands. */
public final class Formula extends NativeResource {
    private Formula(long handle) {
        super(handle, NativeLib::formulaDestroy);
    }

    /** Parse a PrSTL formula such as {@code "always[0, 10](x > 0)"}. */
    public static Formula parse(String formula) throws SentilException {
        return new Formula(NativeLib.formulaParse(formula));
    }
}