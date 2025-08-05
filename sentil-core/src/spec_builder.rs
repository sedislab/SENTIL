//! The premade library of parameterized PrSTL specification templates.

use std::collections::HashMap;

use serde::Deserialize;

/// A complete specification template, deserialized from TOML, YAML, or JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecTemplate {
    /// What the specification is and where it comes from.
    pub metadata: Metadata,
    /// The signals the formula reads.
    #[serde(default)]
    pub variables: Option<HashMap<String, VariableDef>>,
    /// The parameters that fill the formula's placeholders.
    pub parameters: HashMap<String, ParameterDef>,
    /// The formula templates.
    pub formulas: Formulas,
    /// Per-signal noise models.
    #[serde(default)]
    pub noise: Option<HashMap<String, NoiseDef>>,
    /// Per-signal noise-fitting recipes.
    #[serde(default)]
    pub noise_fit: Option<HashMap<String, NoiseFitDef>>,
    /// Recommended verification settings.
    #[serde(default)]
    pub verification: Option<VerificationConfig>,
    /// Named variants that override formulas, parameters, or noise.
    #[serde(default)]
    pub variants: Option<HashMap<String, VariantDef>>,
}

/// What a specification captures and where it is sourced from.
#[derive(Debug, Clone, Deserialize)]
pub struct Metadata {
    /// A human-readable name.
    pub name: String,
    /// The application domain, such as `controls`.
    pub domain: String,
    /// A prose description of the requirement.
    #[serde(default)]
    pub description: Option<String>,
    /// Citations to the standards or papers the specification derives from.
    #[serde(default)]
    pub references: Option<Vec<String>>,
}

/// A signal the formula reads.
#[derive(Debug, Clone, Deserialize)]
pub struct VariableDef {
    /// The unit of the signal.
    #[serde(default)]
    pub unit: Option<String>,
    /// What the signal measures.
    #[serde(default)]
    pub description: Option<String>,
}

/// A parameter that fills a `{name}` placeholder in a formula.
#[derive(Debug, Clone, Deserialize)]
pub struct ParameterDef {
    /// The declared type.
    #[serde(rename = "type")]
    pub param_type: String,
    /// The default value.
    pub default: f64,
    /// The unit of the parameter.
    #[serde(default)]
    pub unit: Option<String>,
    /// An inclusive `[min, max]` range the value must lie in.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    /// What the parameter controls.
    #[serde(default)]
    pub description: Option<String>,
}

/// The formula templates.
#[derive(Debug, Clone, Deserialize)]
pub struct Formulas {
    /// The STL formula template.
    pub deterministic: String,
    /// The PrSTL formula template, if the specification defines one.
    #[serde(default)]
    pub probabilistic: Option<String>,
}

/// A noise model definition for one signal.
#[derive(Debug, Clone, Deserialize)]
pub struct NoiseDef {
    /// The distribution name, such as `Gaussian`.
    pub model: String,
    /// `additive` or `multiplicative`.
    pub interaction: String,
    /// The distribution-specific parameters, keyed by name.
    #[serde(flatten)]
    pub params: HashMap<String, serde_json::Value>,
}

/// A recipe for fitting a noise model from calibration data.
#[derive(Debug, Clone, Deserialize)]
pub struct NoiseFitDef {
    /// The fitting algorithm, such as `fit_gaussian` or `fit_gmm`.
    pub algorithm: String,
    /// The mixture component count, for a Gaussian-mixture fit.
    #[serde(default)]
    pub k: Option<u32>,
    /// The maximum number of fitting iterations.
    #[serde(default)]
    pub max_iters: Option<u32>,
    /// `additive` or `multiplicative`.
    pub interaction: String,
}

/// Recommended verification settings carried with a specification.
#[derive(Debug, Clone, Deserialize)]
pub struct VerificationConfig {
    /// Monte Carlo settings.
    #[serde(default)]
    pub smc: Option<SmcSettings>,
    /// Sequential-test settings.
    #[serde(default)]
    pub sprt: Option<SprtSettings>,
    /// Rare-event splitting settings.
    #[serde(default)]
    pub ams: Option<AmsSettings>,
}

/// Recommended Monte Carlo settings.
#[derive(Debug, Clone, Deserialize)]
pub struct SmcSettings {
    /// The confidence level for the reported interval.
    pub confidence: f64,
    /// The number of samples to draw.
    pub sample_budget: u64,
}

/// Recommended sequential-test settings.
#[derive(Debug, Clone, Deserialize)]
pub struct SprtSettings {
    /// The null-hypothesis probability.
    pub p0: f64,
    /// The alternative-hypothesis probability.
    pub p1: f64,
    /// The Type I error bound.
    pub alpha: f64,
    /// The Type II error bound.
    pub beta: f64,
    /// The cap on samples before the test gives up.
    pub max_samples: usize,
}

/// Recommended rare-event splitting settings.
#[derive(Debug, Clone, Deserialize)]
pub struct AmsSettings {
    /// The particle population.
    pub num_particles: usize,
    /// The cap on simulation steps.
    pub max_steps: usize,
}

/// A named variant that overrides parts of the base template.
#[derive(Debug, Clone, Deserialize)]
pub struct VariantDef {
    /// What the variant changes.
    #[serde(default)]
    pub description: Option<String>,
    /// Formula overrides, applied per direction.
    #[serde(default)]
    pub formulas: Option<VariantFormulas>,
    /// Parameter-default overrides.
    #[serde(default)]
    pub parameters: Option<HashMap<String, VariantParamOverride>>,
    /// Per-signal noise overrides.
    #[serde(default)]
    pub noise: Option<HashMap<String, NoiseDef>>,
}

/// A variant's formula overrides.
#[derive(Debug, Clone, Deserialize)]
pub struct VariantFormulas {
    /// Replaces the base deterministic formula when present.
    #[serde(default)]
    pub deterministic: Option<String>,
    /// Replaces the base probabilistic formula when present.
    #[serde(default)]
    pub probabilistic: Option<String>,
}

/// A variant's override of one parameter's default.
#[derive(Debug, Clone, Deserialize)]
pub struct VariantParamOverride {
    /// The overriding default value.
    pub default: f64,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the parsed parameter values are exact")]

    use super::*;

    const SAMPLE: &str = r#"
[metadata]
name = "Overshoot"
domain = "controls"
references = ["Ogata, Modern Control Engineering, 5th ed., Ch. 5"]

[parameters]
T = { type = "float", default = 30.0, unit = "seconds" }
p = { type = "float", default = 0.95, range = [0.0, 1.0] }

[formulas]
deterministic = "always[0, {T}](output < 1.0)"
probabilistic = "P >= {p}(always[0, {T}](output < 1.0))"

[noise]
output = { model = "Gaussian", mean = 0.0, std_dev = 0.01, interaction = "additive" }
"#;

    #[test]
    fn deserializes_a_toml_template() {
        let template: SpecTemplate = toml::from_str(SAMPLE).unwrap();
        assert_eq!(template.metadata.name, "Overshoot");
        assert_eq!(template.metadata.domain, "controls");
        assert_eq!(template.parameters["T"].default, 30.0);
        assert_eq!(template.parameters["p"].range, Some([0.0, 1.0]));
        assert_eq!(
            template.formulas.deterministic,
            "always[0, {T}](output < 1.0)"
        );
        assert!(template.formulas.probabilistic.is_some());
    }

    #[test]
    fn flattens_distribution_parameters_into_the_noise_map() {
        let template: SpecTemplate = toml::from_str(SAMPLE).unwrap();
        let noise = &template.noise.unwrap()["output"];
        assert_eq!(noise.model, "Gaussian");
        assert_eq!(noise.interaction, "additive");
        assert_eq!(noise.params["std_dev"].as_f64().unwrap(), 0.01);
    }
}