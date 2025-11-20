package io.github.sedislab.sentil;

/**
 * A user-defined simulator over an opaque, fixed-size state, for
 * {@link Synthesis#adaptiveMultilevelSplitting}. Must be thread-safe.
 */
public interface AmsInterface {
    /** The size in bytes of the opaque state. */
    int stateSize();

    /** The initial state for a seed, as {@link #stateSize()} bytes. */
    byte[] initialState(long seed);

    /** The next state given the current state and a seed. */
    byte[] step(byte[] state, long seed);

    /** Whether the run has ended, setting inRareEvent[0] to whether it ended in the rare event. */
    boolean isTerminal(byte[] state, boolean[] inRareEvent);

    /** The score of a state. */
    double score(byte[] state);
}