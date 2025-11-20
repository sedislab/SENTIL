package io.github.sedislab.sentil;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

class SpecsTest {
    @Test
    void registryListsEverySpec() throws Exception {
        List<String> names = SpecBuilder.available();
        assertEquals(54, names.size());
        assertTrue(names.contains("aerospace/airspeed_envelope"));
        for (int i = 1; i < names.size(); i++) {
            assertTrue(names.get(i - 1).compareTo(names.get(i)) < 0);
        }
    }

    @Test
    void buildsAFormulaWithSettings() throws Exception {
        try (SpecBuilder b = new SpecBuilder("aerospace/airspeed_envelope")) {
            String text = b.buildDeterministic();
            assertFalse(text.isEmpty());
            try (Formula f = b.buildFormula()) {
                assertFalse(f.variables().isEmpty());
            }
            assertTrue(b.smcSettings().isPresent());
            assertTrue(b.parametersJson().contains("{"));
        }
    }

    @Test
    void everySpecParsesAndBuilds() throws Exception {
        for (String name : SpecBuilder.available()) {
            try (SpecBuilder b = new SpecBuilder(name); Formula f = b.buildFormula()) {
                assertFalse(f.variables().isEmpty(), name + " has no variables");
            }
        }
    }
}