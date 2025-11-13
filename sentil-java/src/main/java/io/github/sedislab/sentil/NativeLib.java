package io.github.sedislab.sentil;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

/** The loader for the native library and the raw native methods behind the typed classes. */
final class NativeLib {
    private NativeLib() {
    }

    static {
        load();
    }

    private static void load() {
        String dir = "/native/" + classifier() + "/";
        try {
            Path tmp = Files.createTempDirectory("sentil-native");
            tmp.toFile().deleteOnExit();
            Path core = extract(dir, System.mapLibraryName("sentil"), tmp);
            Path shim = extract(dir, System.mapLibraryName("sentil_jni"), tmp);
            System.load(core.toAbsolutePath().toString());
            System.load(shim.toAbsolutePath().toString());
        } catch (IOException e) {
            throw new UncheckedIOException("could not load the SENTIL native library", e);
        }
    }

    private static Path extract(String dir, String name, Path into) throws IOException {
        try (InputStream in = NativeLib.class.getResourceAsStream(dir + name)) {
            if (in == null) {
                throw new IOException("no native library at " + dir + name
                        + "; this jar carries no build for this platform");
            }
            Path out = into.resolve(name);
            Files.copy(in, out, StandardCopyOption.REPLACE_EXISTING);
            out.toFile().deleteOnExit();
            return out;
        }
    }

    private static String classifier() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        String osName;
        if (os.contains("win")) {
            osName = "windows";
        } else if (os.contains("mac") || os.contains("darwin")) {
            osName = "darwin";
        } else {
            osName = "linux";
        }
        String archName;
        if (arch.equals("amd64") || arch.equals("x86_64")) {
            archName = "x86_64";
        } else if (arch.equals("aarch64") || arch.equals("arm64")) {
            archName = "aarch64";
        } else {
            archName = arch;
        }
        return osName + "-" + archName;
    }

    static native int[] version();
}