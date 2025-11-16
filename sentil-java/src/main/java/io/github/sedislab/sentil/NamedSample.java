package io.github.sedislab.sentil;

import java.util.Map;

/** A sample split into the parallel name and value arrays the C ABI takes. */
final class NamedSample {
    final String[] names;
    final double[] values;

    private NamedSample(String[] names, double[] values) {
        this.names = names;
        this.values = values;
    }

    static NamedSample of(Map<String, Double> values) {
        String[] names = new String[values.size()];
        double[] data = new double[values.size()];
        int i = 0;
        for (Map.Entry<String, Double> entry : values.entrySet()) {
            names[i] = entry.getKey();
            data[i] = entry.getValue();
            i++;
        }
        return new NamedSample(names, data);
    }
}