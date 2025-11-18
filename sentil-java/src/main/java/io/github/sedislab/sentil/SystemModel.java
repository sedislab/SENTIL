package io.github.sedislab.sentil;

/**
 * A dynamical system the synthesizer drives. Today it is a linear time-invariant
 * model x_{t+1} = A x_t + B u_t. Pass it to {@link Synthesis#synthesize} or a
 * {@link Controller}. A model owns a native handle, so close it when done.
 */
public final class SystemModel extends NativeResource {
    SystemModel(long handle) {
        super(handle, NativeLib::systemModelDestroy);
    }

    /** A linear model x_{t+1} = A x_t + B u_t. */
    public static SystemModel linear(double[][] a, double[][] b, double[] x0, String[] variables,
            double dt, long horizon) throws SentilException {
        int n = a.length;
        int bCols = b.length == 0 ? 0 : b[0].length;
        return new SystemModel(NativeLib.linearModelCreate(flatten(a, n), n, flatten(b, bCols),
                bCols, x0, variables, dt, horizon));
    }

    private static double[] flatten(double[][] matrix, int cols) {
        double[] flat = new double[matrix.length * cols];
        for (int i = 0; i < matrix.length; i++) {
            System.arraycopy(matrix[i], 0, flat, i * cols, cols);
        }
        return flat;
    }

    /** The total length of the input sequence the synthesizer optimizes. */
    public long inputDimension() {
        return NativeLib.systemModelInputDimension(handle());
    }
}