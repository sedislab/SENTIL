package io.github.sedislab.sentil;

/** The eigendecomposition of a symmetric matrix. */
public final class EigenDecomposition {
    private final double[] values;
    private final double[][] vectors;

    EigenDecomposition(double[] values, double[][] vectors) {
        this.values = values;
        this.vectors = vectors;
    }

    /** The eigenvalues. */
    public double[] values() {
        return values;
    }

    /** The eigenvectors as rows; row j corresponds to eigenvalue j. */
    public double[][] vectors() {
        return vectors;
    }
}