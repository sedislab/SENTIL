package io.github.sedislab.sentil;

import java.lang.ref.Cleaner;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.LongConsumer;

/** The base of every class that owns a native SENTIL handle. */
public abstract class NativeResource implements AutoCloseable {
    private static final Cleaner CLEANER = Cleaner.create();

    private static final class State implements Runnable {
        private final AtomicLong handle;
        private final LongConsumer destroy;

        State(long handle, LongConsumer destroy) {
            this.handle = new AtomicLong(handle);
            this.destroy = destroy;
        }

        @Override
        public void run() {
            long h = handle.getAndSet(0L);
            if (h != 0L) {
                destroy.accept(h);
            }
        }
    }

    private final State state;
    private final Cleaner.Cleanable cleanable;

    protected NativeResource(long handle, LongConsumer destroy) {
        if (handle == 0L) {
            throw new IllegalStateException("native handle was null");
        }
        this.state = new State(handle, destroy);
        this.cleanable = CLEANER.register(this, state);
    }

    final long handle() {
        long h = state.handle.get();
        if (h == 0L) {
            throw new IllegalStateException(getClass().getSimpleName() + " is closed");
        }
        return h;
    }

    final long disown() {
        return state.handle.getAndSet(0L);
    }

    /** Free the native handle. Idempotent. */
    @Override
    public final void close() {
        cleanable.clean();
    }
}