//! The premade library of parameterized PrSTL specification templates.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use rust_embed::RustEmbed;
use serde::Deserialize;

use crate::error::{Error, Result};

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

#[derive(RustEmbed)]
#[folder = "../specifications/"]
struct EmbeddedSpecs;

/// Loads and caches specification templates by name.
#[derive(Debug, Clone, Default)]
pub struct SpecRegistry {
    cache: Arc<RwLock<HashMap<String, SpecTemplate>>>,
}

impl SpecRegistry {
    /// The process-wide registry.
    #[must_use]
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<SpecRegistry> = OnceLock::new();
        REGISTRY.get_or_init(SpecRegistry::default)
    }

    /// Resolves `name` to a template, caching the result.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the name is empty or cannot be resolved,
    /// read, or parsed.
    pub fn get(&self, name: &str) -> Result<SpecTemplate> {
        if name.is_empty() {
            return Err(spec_error(
                "specification name cannot be empty; use a library key like \
                 'controls/overshoot' or a path like './my_spec.toml'"
                    .to_owned(),
            ));
        }
        if let Some(template) = self.cached(name)? {
            return Ok(template);
        }
        let template = if is_filesystem_path(name) {
            load_from_filesystem(name)?
        } else {
            load_from_env_or_embedded(name)?
        };
        self.store(name, template.clone())?;
        Ok(template)
    }

    /// Loads a template straight from a file path, bypassing the cache.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the file cannot be read or parsed.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<SpecTemplate> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            spec_error(format!(
                "failed to read specification file '{}': {e}",
                path.display()
            ))
        })?;
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        parse_template(&content, extension, &path.display().to_string())
    }

    /// The names of the embedded library specifications, sorted.
    #[must_use]
    pub fn available(&self) -> Vec<String> {
        let mut names: Vec<String> = EmbeddedSpecs::iter()
            .filter(|key| {
                matches!(
                    Path::new(key.as_ref()).extension().and_then(|e| e.to_str()),
                    Some("toml" | "yaml" | "json")
                )
            })
            .map(|key| strip_extension(&key).to_owned())
            .collect();
        names.sort();
        names
    }

    fn cached(&self, name: &str) -> Result<Option<SpecTemplate>> {
        let cache = self.cache.read().map_err(|_| lock_poisoned())?;
        Ok(cache.get(name).cloned())
    }

    fn store(&self, name: &str, template: SpecTemplate) -> Result<()> {
        let mut cache = self.cache.write().map_err(|_| lock_poisoned())?;
        cache.insert(name.to_owned(), template);
        Ok(())
    }
}

/// Parses a template from `content`, choosing the format by `extension`.
pub(crate) fn parse_template(content: &str, extension: &str, source: &str) -> Result<SpecTemplate> {
    match extension {
        "toml" => toml::from_str(content)
            .map_err(|e| spec_error(format!("failed to parse TOML from '{source}': {e}"))),
        "json" => serde_json::from_str(content)
            .map_err(|e| spec_error(format!("failed to parse JSON from '{source}': {e}"))),
        "yaml" | "yml" => serde_yaml::from_str(content)
            .map_err(|e| spec_error(format!("failed to parse YAML from '{source}': {e}"))),
        _ => toml::from_str(content)
            .or_else(|_| serde_json::from_str(content))
            .or_else(|_| serde_yaml::from_str(content))
            .map_err(|_: serde_yaml::Error| {
                spec_error(format!(
                    "could not parse '{source}' as TOML, JSON, or YAML; use a recognized file \
                     extension (.toml, .yaml, .json) for a clearer error"
                ))
            }),
    }
}

pub(crate) fn spec_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "specification",
        message,
    }
}

fn lock_poisoned() -> Error {
    spec_error("the specification registry lock is poisoned".to_owned())
}

fn is_filesystem_path(name: &str) -> bool {
    if name.starts_with("./") || name.starts_with('/') {
        return true;
    }
    matches!(
        Path::new(name).extension().and_then(|e| e.to_str()),
        Some("toml" | "yaml" | "yml" | "json")
    )
}

fn extension_of(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}

fn strip_extension(key: &str) -> &str {
    key.rsplit_once('.').map_or(key, |(stem, _)| stem)
}

fn load_from_filesystem(name: &str) -> Result<SpecTemplate> {
    let content = std::fs::read_to_string(name)
        .map_err(|e| spec_error(format!("failed to read specification file '{name}': {e}")))?;
    parse_template(&content, extension_of(name), name)
}

fn load_from_env_or_embedded(name: &str) -> Result<SpecTemplate> {
    if let Ok(dir) = std::env::var("SENTIL_SPECS_DIR") {
        let base = Path::new(&dir);
        for candidate in [
            base.join(format!("{name}.toml")),
            base.join(format!("{name}.yaml")),
            base.join(format!("{name}.json")),
            base.join(name),
        ] {
            if candidate.is_file() {
                let content = std::fs::read_to_string(&candidate).map_err(|e| {
                    spec_error(format!(
                        "found '{}' via SENTIL_SPECS_DIR but could not read it: {e}",
                        candidate.display()
                    ))
                })?;
                let extension = candidate.extension().and_then(|e| e.to_str()).unwrap_or("");
                return parse_template(&content, extension, &candidate.display().to_string());
            }
        }
    }
    for key in [
        name.to_owned(),
        format!("{name}.toml"),
        format!("{name}.yaml"),
        format!("{name}.json"),
    ] {
        if let Some(file) = EmbeddedSpecs::get(&key) {
            let content = std::str::from_utf8(&file.data).map_err(|e| {
                spec_error(format!(
                    "embedded specification '{key}' is not valid UTF-8: {e}"
                ))
            })?;
            return parse_template(content, extension_of(&key), &key);
        }
    }
    Err(spec_error(format!(
        "specification '{name}' not found in the embedded library or SENTIL_SPECS_DIR"
    )))
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

    #[test]
    fn every_embedded_spec_parses() {
        let registry = SpecRegistry::default();
        let names = registry.available();
        assert!(names.len() >= 14, "found only {} specs", names.len());
        for name in &names {
            registry
                .get(name)
                .unwrap_or_else(|e| panic!("embedded spec '{name}' failed to parse: {e}"));
        }
    }

    #[test]
    fn resolves_a_known_control_spec() {
        let template = SpecRegistry::default().get("controls/overshoot").unwrap();
        assert_eq!(template.metadata.domain, "controls");
        assert!(template.formulas.probabilistic.is_some());
    }

    #[test]
    fn the_cache_returns_the_same_template_twice() {
        let registry = SpecRegistry::default();
        let first = registry.get("controls/settling_time").unwrap();
        let second = registry.get("controls/settling_time").unwrap();
        assert_eq!(first.metadata.name, second.metadata.name);
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert!(SpecRegistry::default().get("").is_err());
    }

    #[test]
    fn an_unknown_name_is_rejected() {
        assert!(SpecRegistry::default()
            .get("controls/does_not_exist")
            .is_err());
    }
}