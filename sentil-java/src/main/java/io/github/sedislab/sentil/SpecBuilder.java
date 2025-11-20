package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;
import java.util.Optional;

/** The specifications-library loader. */
public final class SpecBuilder extends NativeResource {
    SpecBuilder(long handle) {
        super(handle, NativeLib::specBuilderDestroy);
    }

    /** A builder for the named spec from the embedded registry. */
    public SpecBuilder(String name) throws SentilException {
        this(NativeLib.specBuilderCreate(name));
    }

    /** The names of every embedded specification, sorted. */
    public static List<String> available() throws SentilException {
        return Arrays.asList(NativeLib.specRegistryAvailable());
    }

    /** A builder loaded from a spec template file. */
    public static SpecBuilder fromFile(String path) throws SentilException {
        return new SpecBuilder(NativeLib.specBuilderFromFile(path));
    }

    /** Select a named variant, consuming this builder and returning a new one. */
    public SpecBuilder withVariant(String variant) throws SentilException {
        return new SpecBuilder(NativeLib.specBuilderWithVariant(consume(), variant));
    }

    /** Override a parameter, consuming this builder and returning a new one. */
    public SpecBuilder withParam(String name, double value) throws SentilException {
        return new SpecBuilder(NativeLib.specBuilderWithParam(consume(), name, value));
    }

    /** The variant names the spec offers, sorted. */
    public List<String> availableVariants() throws SentilException {
        return Arrays.asList(NativeLib.specBuilderAvailableVariants(handle()));
    }

    /** The deterministic formula text with the parameters filled in. */
    public String buildDeterministic() throws SentilException {
        return NativeLib.specBuilderBuildDeterministic(handle());
    }

    /** The probabilistic formula text with the parameters filled in. */
    public String buildProbabilistic() throws SentilException {
        return NativeLib.specBuilderBuildProbabilistic(handle());
    }

    /** The deterministic formula as a handle. */
    public Formula buildFormula() throws SentilException {
        return new Formula(NativeLib.specBuilderBuildFormula(handle()));
    }

    /** The probabilistic formula as a handle. */
    public Formula buildProbabilisticFormula() throws SentilException {
        return new Formula(NativeLib.specBuilderBuildProbabilisticFormula(handle()));
    }

    /** A lifting registry built from the spec's resolved noise models. */
    public LiftingRegistry buildLiftingRegistry() throws SentilException {
        return new LiftingRegistry(NativeLib.specBuilderBuildLiftingRegistry(handle()));
    }

    /** The resolved parameters as a JSON object. */
    public String parametersJson() throws SentilException {
        return NativeLib.specBuilderParametersJson(handle());
    }

    /** A monitor preloaded with the spec's recommended settings, consuming this builder. */
    public Monitor intoMonitor() throws SentilException {
        return new Monitor(NativeLib.specBuilderIntoMonitor(consume()));
    }

    /** The SMC settings the spec recommends, or empty when it carries none. */
    public Optional<SpecSmcSettings> smcSettings() {
        return Optional.ofNullable(NativeLib.specBuilderSmcSettings(handle()));
    }

    /** The SPRT settings the spec recommends, or empty when it carries none. */
    public Optional<SpecSprtSettings> sprtSettings() {
        return Optional.ofNullable(NativeLib.specBuilderSprtSettings(handle()));
    }

    /** The rare-event settings the spec recommends, or empty when it carries none. */
    public Optional<SpecAmsSettings> amsSettings() {
        return Optional.ofNullable(NativeLib.specBuilderAmsSettings(handle()));
    }
}