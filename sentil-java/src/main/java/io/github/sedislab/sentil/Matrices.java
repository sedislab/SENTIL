package io.github.sedislab.sentil;

/** Row-major matrix marshalling shared by the model and the numeric solvers. */
final class Matrices {
    private Matrices() {
    }

    /** Flatten row-major. */
    static double[] flatten(double[][] matrix, int cols) throws SentilException {
        double[] flat = new double[matrix.length * cols];
        for (int i = 0; i < matrix.length; i++) {
            double[] row = matrix[i];
            if (row == null || row.length != cols) {
                throw new EvaluationException("matrix row " + i + " has length "
                        + (row == null ? "null" : Integer.toString(row.length)) + ", expected "
                        + cols, ErrorCode.INVALID_CONFIG.code());
            }
            System.arraycopy(row, 0, flat, i * cols, cols);
        }
        return flat;
    }
}