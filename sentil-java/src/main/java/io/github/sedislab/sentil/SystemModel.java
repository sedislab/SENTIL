package io.github.sedislab.sentil;

/**
 * A dynamical system the synthesizer drives, either linear x_{t+1} = A x_t + B u_t or
 * a host rollout.
 */
public final class SystemModel extends NativeResource {
    private final long box;

    SystemModel(long handle) {
        super(handle, NativeLib::systemModelDestroy);
        this.box = 0L;
    }

    private SystemModel(long handle, long box) {
        super(handle, h -> {
            NativeLib.systemModelDestroy(h);
            NativeLib.freeModelBox(box);
        });
        this.box = box;
    }

    /** A linear model x_{t+1} = A x_t + B u_t. */
    public static SystemModel linear(double[][] a, double[][] b, double[] x0, String[] variables,
            double dt, long horizon) throws SentilException {
        int n = a.length;
        if (x0.length != n || variables.length != n || b.length != n) {
            throw new EvaluationException(
                    "A is n-by-n, B has n rows, and x0 and variables have length n",
                    ErrorCode.INVALID_CONFIG.code());
        }
        int bCols = (b.length == 0 || b[0] == null) ? 0 : b[0].length;
        return new SystemModel(NativeLib.linearModelCreate(Matrices.flatten(a, n), n,
                Matrices.flatten(b, bCols), bCols, x0, variables, dt, horizon));
    }

    /** A model whose rollout is a host function. It must be thread-safe. */
    public static SystemModel custom(String[] variables, double dt, long horizon,
            double[] initialState, long inputDimension, Rollout rollout) throws SentilException {
        long[] result = NativeLib.systemModelCreateCustom(variables, dt, horizon, initialState,
                inputDimension, rollout);
        return new SystemModel(result[0], result[1]);
    }

    /** The total length of the input sequence the synthesizer optimizes. */
    public long inputDimension() {
        return NativeLib.systemModelInputDimension(handle());
    }

    long modelBox() {
        return box;
    }

    void rethrowCallbackError() {
        if (box != 0L) {
            NativeLib.rethrowModelError(box);
        }
    }
}