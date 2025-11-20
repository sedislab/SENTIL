package io.github.sedislab.sentil;

/** A receding-horizon controller that emits a control input within a hard deadline. */
public final class Controller extends NativeResource {
    private final long inputWidth;
    private final long modelBox;

    private Controller(long handle, long modelBox, long inputWidth) {
        super(handle, h -> {
            NativeLib.controllerDestroy(h);
            if (modelBox != 0L) {
                NativeLib.freeModelBox(modelBox);
            }
        });
        this.inputWidth = inputWidth;
        this.modelBox = modelBox;
    }

    /**
     * A controller over the model and spec, both consumed, with budgetNs the per-step
     * deadline in nanoseconds.
     */
    public Controller(SystemModel model, Formula spec, long inputWidth, long budgetNs)
            throws SentilException {
        this(model, spec, inputWidth, budgetNs, null, null);
    }

    /** A controller with optional bounds and smoothing on the planned input. */
    public Controller(SystemModel model, Formula spec, long inputWidth, long budgetNs, Bounds bounds,
            SmoothConfig smooth) throws SentilException {
        this(build(model, spec, inputWidth, budgetNs, bounds, smooth), model.modelBox(), inputWidth);
    }

    private static long build(SystemModel model, Formula spec, long inputWidth, long budgetNs,
            Bounds bounds, SmoothConfig smooth) throws SentilException {
        long modelHandle = model.handle();
        long specHandle = spec.handle();
        long box = model.modelBox();
        model.disown();
        spec.disown();
        long boundsHandle = bounds == null ? 0L : bounds.handle();
        double temperature = smooth == null ? 0.0 : smooth.temperature;
        int kind = smooth == null ? 0 : smooth.kind.code();
        try {
            return NativeLib.controllerCreate(modelHandle, specHandle, inputWidth, budgetNs,
                    boundsHandle, temperature, kind, smooth != null);
        } catch (SentilException error) {
            if (box != 0L) {
                NativeLib.freeModelBox(box);
            }
            throw error;
        }
    }

    /** Plan from the current state and return the first control input. */
    public double[] control(double[] state) throws SentilException {
        double[] input = NativeLib.controllerControl(handle(), state, inputWidth);
        if (modelBox != 0L) {
            NativeLib.rethrowModelError(modelBox);
        }
        return input;
    }
}