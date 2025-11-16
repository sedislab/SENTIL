package io.github.sedislab.sentil;

/** A monitor's configuration. */
public final class Config extends NativeResource {
    Config(long handle) {
        super(handle, NativeLib::configDestroy);
    }

    /** A configuration with the default discrete time mode. */
    public Config() throws SentilException {
        this(TimeMode.DISCRETE);
    }

    /** A configuration with the given time mode. */
    public Config(TimeMode time) throws SentilException {
        this(NativeLib.configCreate());
        if (time != TimeMode.DISCRETE) {
            NativeLib.configSetTime(handle(), time.code());
        }
    }

    /** The time mode the monitor will use. */
    public TimeMode timeMode() {
        return TimeMode.fromCode(NativeLib.configTimeMode(handle()));
    }
}