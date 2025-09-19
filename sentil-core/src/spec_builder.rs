//! The premade library of parameterized PrSTL specification templates.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use rust_embed::RustEmbed;
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::stats::{LiftingRegistry, NoiseInteraction, NoiseModel};
use crate::Formula;

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

    /// Resolves `name` and wraps it in a [`SpecBuilder`] ready for parameters.
    ///
    /// # Errors
    ///
    /// As [`get`](Self::get).
    pub fn builder(&self, name: &str) -> Result<SpecBuilder> {
        Ok(SpecBuilder::new(self.get(name)?))
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

/// Instantiates a [`SpecTemplate`] into a concrete formula by filling its parameters.
///
/// ```
/// use sentil::spec_builder::SpecRegistry;
///
/// let formula = SpecRegistry::global()
///     .builder("controls/overshoot")?
///     .with_param("max_overshoot", 0.1)?
///     .build_deterministic()?;
/// assert!(formula.contains("0.1"));
/// # Ok::<(), sentil::Error>(())
/// ```
pub struct SpecBuilder {
    template: SpecTemplate,
    active_variant: Option<String>,
    param_overrides: HashMap<String, f64>,
}

impl SpecBuilder {
    /// Wraps a template for instantiation.
    #[must_use]
    pub fn new(template: SpecTemplate) -> Self {
        Self {
            template,
            active_variant: None,
            param_overrides: HashMap::new(),
        }
    }

    /// The underlying template.
    #[must_use]
    pub fn template(&self) -> &SpecTemplate {
        &self.template
    }

    /// Selects a named variant.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the variant is unknown or the template
    /// defines no variants.
    pub fn with_variant(mut self, variant: &str) -> Result<Self> {
        match &self.template.variants {
            Some(variants) if variants.contains_key(variant) => {
                self.active_variant = Some(variant.to_owned());
                Ok(self)
            }
            Some(variants) => {
                let mut available: Vec<&str> = variants.keys().map(String::as_str).collect();
                available.sort_unstable();
                Err(spec_error(format!(
                    "variant '{variant}' not found in '{}'; available: [{}]",
                    self.template.metadata.name,
                    available.join(", ")
                )))
            }
            None => Err(spec_error(format!(
                "'{}' defines no variants",
                self.template.metadata.name
            ))),
        }
    }

    /// Overrides a parameter's value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `value` is not finite, the parameter is
    /// unknown, or the value falls outside the parameter's declared range.
    pub fn with_param(mut self, name: &str, value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(spec_error(format!(
                "parameter '{name}' must be finite, got {value}"
            )));
        }
        let Some(def) = self.template.parameters.get(name) else {
            let mut available: Vec<&str> = self
                .template
                .parameters
                .keys()
                .map(String::as_str)
                .collect();
            available.sort_unstable();
            return Err(spec_error(format!(
                "parameter '{name}' is not defined in '{}'; available: [{}]",
                self.template.metadata.name,
                available.join(", ")
            )));
        };
        if let Some([min, max]) = def.range {
            if value < min || value > max {
                return Err(spec_error(format!(
                    "parameter '{name}' value {value} is outside the allowed range [{min}, {max}]"
                )));
            }
        }
        self.param_overrides.insert(name.to_owned(), value);
        Ok(self)
    }

    /// The deterministic STL formula with the chosen parameters substituted.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the formula leaves a placeholder
    /// unresolved.
    pub fn build_deterministic(&self) -> Result<String> {
        self.resolve_formula(true)
    }

    /// The probabilistic PrSTL formula with the chosen parameters substituted.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the template has no probabilistic formula
    /// or the formula leaves a placeholder unresolved.
    pub fn build_probabilistic(&self) -> Result<String> {
        self.resolve_formula(false)
    }

    /// The deterministic formula parsed into a [`Formula`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] for an unresolved placeholder, or a parse
    /// error naming the column.
    pub fn build_formula(&self) -> Result<Formula> {
        Formula::parse(&self.build_deterministic()?)
    }

    /// The probabilistic formula parsed into a [`Formula`].
    ///
    /// # Errors
    ///
    /// As [`build_formula`](Self::build_formula), for the probabilistic formula.
    pub fn build_probabilistic_formula(&self) -> Result<Formula> {
        Formula::parse(&self.build_probabilistic()?)
    }

    /// The available variant names, sorted.
    #[must_use]
    pub fn available_variants(&self) -> Vec<&str> {
        self.template
            .variants
            .as_ref()
            .map_or_else(Vec::new, |variants| {
                let mut names: Vec<&str> = variants.keys().map(String::as_str).collect();
                names.sort_unstable();
                names
            })
    }

    /// The parameters and their currently resolved values.
    #[must_use]
    pub fn parameters(&self) -> HashMap<String, f64> {
        self.resolve_parameters()
    }

    /// The noise models in force, or `None` if the template defines none.
    #[must_use]
    pub fn resolved_noise(&self) -> Option<HashMap<String, NoiseDef>> {
        let mut noise = self.template.noise.clone()?;
        if let Some(overrides) = self.variant().and_then(|v| v.noise.as_ref()) {
            for (variable, def) in overrides {
                noise.insert(variable.clone(), def.clone());
            }
        }
        Some(noise)
    }

    /// Builds a lifting registry from the resolved noise models.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if a noise model or interaction is malformed.
    pub fn build_lifting_registry(&self) -> Result<LiftingRegistry> {
        let mut registry = LiftingRegistry::new();
        if let Some(noise) = self.resolved_noise() {
            for (variable, def) in &noise {
                let model = noise_model_from_def(def)?;
                let interaction = parse_interaction(&def.interaction)?;
                registry.register(variable, model, interaction);
            }
        }
        Ok(registry)
    }

    /// The recommended Monte Carlo settings, if the template carries any.
    #[must_use]
    pub fn smc_settings(&self) -> Option<&SmcSettings> {
        self.template.verification.as_ref()?.smc.as_ref()
    }

    /// The recommended sequential-test settings, if the template carries any.
    #[must_use]
    pub fn sprt_settings(&self) -> Option<&SprtSettings> {
        self.template.verification.as_ref()?.sprt.as_ref()
    }

    /// The recommended rare-event settings, if the template carries any.
    #[must_use]
    pub fn ams_settings(&self) -> Option<&AmsSettings> {
        self.template.verification.as_ref()?.ams.as_ref()
    }

    /// Builds a [`Monitor`](crate::Monitor) preloaded with the template's
    /// recommended settings.
    ///
    /// # Errors
    ///
    /// Propagates any error from instantiating the formula.
    pub fn into_monitor(self) -> Result<crate::Monitor> {
        let formula = self.build_formula()?;
        let mut config = crate::MonitorConfig::new();
        if let Some(smc) = self.smc_settings() {
            config = config.smc(crate::SmcConfig {
                samples: smc.sample_budget,
                confidence: smc.confidence,
                ..crate::SmcConfig::default()
            });
        }
        if let Some(ams) = self.ams_settings() {
            config = config.rare(crate::RareEventConfig {
                particles: ams.num_particles,
                ..crate::RareEventConfig::default()
            });
        }
        Ok(crate::Monitor::from_formula(formula, config))
    }

    /// The per-signal noise-fitting recipes, if the template carries any.
    #[must_use]
    pub fn noise_fit(&self) -> Option<&HashMap<String, NoiseFitDef>> {
        self.template.noise_fit.as_ref()
    }

    fn variant(&self) -> Option<&VariantDef> {
        let key = self.active_variant.as_ref()?;
        self.template.variants.as_ref()?.get(key)
    }

    fn resolve_parameters(&self) -> HashMap<String, f64> {
        let mut resolved: HashMap<String, f64> = self
            .template
            .parameters
            .iter()
            .map(|(name, def)| (name.clone(), def.default))
            .collect();
        if let Some(overrides) = self.variant().and_then(|v| v.parameters.as_ref()) {
            for (name, over) in overrides {
                resolved.insert(name.clone(), over.default);
            }
        }
        for (name, value) in &self.param_overrides {
            resolved.insert(name.clone(), *value);
        }
        resolved
    }

    fn resolve_formula(&self, deterministic: bool) -> Result<String> {
        if let Some(formulas) = self.variant().and_then(|v| v.formulas.as_ref()) {
            let overridden = if deterministic {
                &formulas.deterministic
            } else {
                &formulas.probabilistic
            };
            if let Some(formula) = overridden {
                return self.interpolate(formula, &self.resolve_parameters());
            }
        }
        let base = if deterministic {
            self.template.formulas.deterministic.clone()
        } else {
            self.template
                .formulas
                .probabilistic
                .clone()
                .ok_or_else(|| {
                    spec_error(format!(
                        "'{}' does not define a probabilistic formula",
                        self.template.metadata.name
                    ))
                })?
        };
        self.interpolate(&base, &self.resolve_parameters())
    }

    fn interpolate(&self, template: &str, params: &HashMap<String, f64>) -> Result<String> {
        let mut formula = template.to_owned();
        for (key, value) in params {
            formula = formula.replace(&format!("{{{key}}}"), &format_param(*value));
        }
        if let Some(start) = formula.find('{') {
            if let Some(end) = formula[start..].find('}') {
                let unresolved = &formula[start + 1..start + end];
                return Err(spec_error(format!(
                    "unresolved parameter '{{{unresolved}}}' in the formula for '{}'; it appears \
                     in the formula but is not defined in [parameters]",
                    self.template.metadata.name
                )));
            }
        }
        Ok(formula)
    }
}

/// Formats a parameter value as a float literal the STL parser accepts.
#[allow(
    clippy::float_cmp,
    reason = "testing whether a float is integer-valued is an exact comparison by design"
)]
fn format_param(value: f64) -> String {
    if value == value.floor() && value.abs() < 1e15 {
        format!("{value:.1}")
    } else {
        let formatted = format!("{value:.10}");
        let trimmed = formatted.trim_end_matches('0');
        if trimmed.ends_with('.') {
            format!("{trimmed}0")
        } else {
            trimmed.to_owned()
        }
    }
}

/// Builds a [`NoiseModel`] from a template's noise definition.
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if the model is unknown, a required parameter
/// is missing or non-numeric, or the constructed model is invalid.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the binomial count is validated as a non-negative integer in u64 range before the cast"
)]
pub fn noise_model_from_def(def: &NoiseDef) -> Result<NoiseModel> {
    let params = &def.params;
    match def.model.to_lowercase().as_str() {
        "dirac" => NoiseModel::dirac(require_f64(params, "value", "Dirac")?),
        "gaussian" | "normal" => NoiseModel::gaussian(
            optional_f64(params, "mean").unwrap_or(0.0),
            require_f64(params, "std_dev", "Gaussian")?,
        ),
        "uniform" => NoiseModel::uniform(
            require_f64(params, "low", "Uniform")?,
            require_f64(params, "high", "Uniform")?,
        ),
        "lognormal" | "log_normal" => NoiseModel::log_normal(
            require_f64(params, "mu", "LogNormal")?,
            require_f64(params, "sigma", "LogNormal")?,
        ),
        "exponential" => NoiseModel::exponential(require_f64(params, "lambda", "Exponential")?),
        "gamma" => NoiseModel::gamma(
            require_f64(params, "shape", "Gamma")?,
            require_f64(params, "scale", "Gamma")?,
        ),
        "beta" => NoiseModel::beta(
            require_f64(params, "alpha", "Beta")?,
            require_f64(params, "beta", "Beta")?,
        ),
        "weibull" => NoiseModel::weibull(
            require_f64(params, "shape", "Weibull")?,
            require_f64(params, "scale", "Weibull")?,
        ),
        "rayleigh" => NoiseModel::rayleigh(require_f64(params, "scale", "Rayleigh")?),
        "gumbel" => NoiseModel::gumbel(
            require_f64(params, "location", "Gumbel")?,
            require_f64(params, "scale", "Gumbel")?,
        ),
        "cauchy" => NoiseModel::cauchy(
            require_f64(params, "location", "Cauchy")?,
            require_f64(params, "scale", "Cauchy")?,
        ),
        "studentt" | "student_t" => NoiseModel::student_t(
            require_f64(params, "df", "StudentT")?,
            optional_f64(params, "location").unwrap_or(0.0),
            optional_f64(params, "scale").unwrap_or(1.0),
        ),
        "truncatednormal" | "truncated_normal" => NoiseModel::truncated_normal(
            optional_f64(params, "mean").unwrap_or(0.0),
            require_f64(params, "std_dev", "TruncatedNormal")?,
            require_f64(params, "lower", "TruncatedNormal")?,
            require_f64(params, "upper", "TruncatedNormal")?,
        ),
        "poisson" => NoiseModel::poisson(require_f64(params, "lambda", "Poisson")?),
        "binomial" => {
            let n = require_f64(params, "n", "Binomial")?;
            if n < 0.0 || n.fract() != 0.0 || n >= 2.0_f64.powi(64) {
                return Err(spec_error(format!(
                    "binomial 'n' must be a non-negative integer that fits a u64, got {n}"
                )));
            }
            NoiseModel::binomial(n as u64, require_f64(params, "p", "Binomial")?)
        }
        other => Err(spec_error(format!(
            "unknown noise model '{other}'; supported: dirac, gaussian, uniform, lognormal, \
             exponential, gamma, beta, weibull, rayleigh, gumbel, cauchy, studentt, \
             truncatednormal, poisson, binomial. For bootstrap or mixture models, register them \
             on a lifting registry directly"
        ))),
    }
}

fn parse_interaction(interaction: &str) -> Result<NoiseInteraction> {
    match interaction.to_lowercase().as_str() {
        "additive" => Ok(NoiseInteraction::Additive),
        "multiplicative" => Ok(NoiseInteraction::Multiplicative),
        other => Err(spec_error(format!(
            "unknown noise interaction '{other}'; use 'additive' or 'multiplicative'"
        ))),
    }
}

fn require_f64(params: &HashMap<String, serde_json::Value>, key: &str, model: &str) -> Result<f64> {
    params
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| {
            let mut keys: Vec<&str> = params.keys().map(String::as_str).collect();
            keys.sort_unstable();
            spec_error(format!(
                "noise model '{model}' needs a numeric parameter '{key}'; given [{}]",
                keys.join(", ")
            ))
        })
}

fn optional_f64(params: &HashMap<String, serde_json::Value>, key: &str) -> Option<f64> {
    params.get(key).and_then(serde_json::Value::as_f64)
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

    #[test]
    fn builds_a_deterministic_formula_with_defaults() {
        let formula = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .build_deterministic()
            .unwrap();
        assert_eq!(formula, "always[0, 30.0](output - reference < 0.05 * 1.0)");
    }

    #[test]
    fn an_overridden_parameter_appears_in_the_formula() {
        let formula = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .with_param("max_overshoot", 0.1)
            .unwrap()
            .build_deterministic()
            .unwrap();
        assert!(formula.contains("0.1 * 1.0"), "{formula}");
    }

    #[test]
    fn an_out_of_range_parameter_is_rejected() {
        let result = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .with_param("p", 1.5);
        assert!(result.is_err());
    }

    #[test]
    fn an_unknown_parameter_is_rejected() {
        let result = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .with_param("nonexistent", 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn a_variant_overrides_the_formula() {
        let formula = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .with_variant("step_down")
            .unwrap()
            .build_deterministic()
            .unwrap();
        assert_eq!(formula, "always[0, 30.0](reference - output < 0.05 * 1.0)");
    }

    #[test]
    fn an_unknown_variant_is_rejected() {
        let result = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .with_variant("sideways");
        assert!(result.is_err());
    }

    #[test]
    fn every_instantiated_formula_parses() {
        use crate::Formula;

        let registry = SpecRegistry::default();
        for name in registry.available() {
            let builder = registry.builder(&name).unwrap();
            let deterministic = builder.build_deterministic().unwrap();
            Formula::parse(&deterministic).unwrap_or_else(|e| {
                panic!("{name} deterministic '{deterministic}' did not parse: {e}")
            });
            let probabilistic = builder.build_probabilistic().unwrap();
            Formula::parse(&probabilistic).unwrap_or_else(|e| {
                panic!("{name} probabilistic '{probabilistic}' did not parse: {e}")
            });
        }
    }

    #[test]
    fn every_variant_formula_parses() {
        use crate::Formula;

        let registry = SpecRegistry::default();
        for name in registry.available() {
            let variants: Vec<String> = registry
                .builder(&name)
                .unwrap()
                .available_variants()
                .iter()
                .map(|v| (*v).to_owned())
                .collect();
            for variant in variants {
                let builder = registry
                    .builder(&name)
                    .unwrap()
                    .with_variant(&variant)
                    .unwrap();
                let deterministic = builder.build_deterministic().unwrap();
                Formula::parse(&deterministic).unwrap_or_else(|e| {
                    panic!("{name}/{variant} deterministic '{deterministic}' did not parse: {e}")
                });
                let probabilistic = builder.build_probabilistic().unwrap();
                Formula::parse(&probabilistic).unwrap_or_else(|e| {
                    panic!("{name}/{variant} probabilistic '{probabilistic}' did not parse: {e}")
                });
            }
        }
    }

    #[test]
    fn builds_a_lifting_registry_from_the_spec_noise() {
        let registry = SpecRegistry::default()
            .builder("controls/disturbance_rejection")
            .unwrap()
            .build_lifting_registry()
            .unwrap();
        let mut variables = registry.variables();
        variables.sort_unstable();
        assert_eq!(variables, vec!["disturbance", "output"]);
    }

    #[test]
    fn a_spec_without_noise_yields_an_empty_registry() {
        let template: SpecTemplate = toml::from_str(
            "[metadata]\nname = \"N\"\ndomain = \"d\"\n[parameters]\n\
             [formulas]\ndeterministic = \"x > 0\"",
        )
        .unwrap();
        assert!(SpecBuilder::new(template)
            .build_lifting_registry()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn maps_a_gaussian_noise_definition() {
        let def = NoiseDef {
            model: "Gaussian".to_owned(),
            interaction: "additive".to_owned(),
            params: HashMap::from([("std_dev".to_owned(), serde_json::Value::from(0.5))]),
        };
        let model = noise_model_from_def(&def).unwrap();
        let debug = format!("{model:?}");
        assert!(
            debug.contains("Gaussian") && debug.contains("std_dev: 0.5"),
            "{debug}"
        );
    }

    #[test]
    fn an_unknown_noise_model_is_rejected() {
        let def = NoiseDef {
            model: "wibble".to_owned(),
            interaction: "additive".to_owned(),
            params: HashMap::new(),
        };
        assert!(noise_model_from_def(&def).is_err());
    }

    #[test]
    fn noise_interaction_parses_case_insensitively() {
        assert!(parse_interaction("Multiplicative").is_ok());
        assert!(parse_interaction("sideways").is_err());
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "the recommended confidence is an exact literal")]
    fn into_monitor_preloads_the_recommended_settings() {
        let monitor = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .into_monitor()
            .unwrap();
        let smc = monitor.config().smc_config();
        assert_eq!(smc.samples, 1000);
        assert_eq!(smc.confidence, 0.95);
    }

    #[test]
    fn reads_the_recommended_verification_settings() {
        let builder = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap();
        let smc = builder.smc_settings().unwrap();
        assert_eq!(smc.confidence, 0.95);
        assert_eq!(smc.sample_budget, 1000);
    }

    #[test]
    fn every_spec_evaluates_on_a_trace() {
        use crate::{Formula, Trace};

        let registry = SpecRegistry::default();
        for name in registry.available() {
            let template = registry.get(&name).unwrap();
            let text = registry
                .builder(&name)
                .unwrap()
                .build_deterministic()
                .unwrap();
            let formula = Formula::parse(&text).unwrap();

            let times: Vec<f64> = (0..=80).map(f64::from).collect();
            let mut trace = Trace::new(times.clone()).unwrap();
            if let Some(variables) = &template.variables {
                for variable in variables.keys() {
                    trace.add_signal(variable, vec![0.5; times.len()]).unwrap();
                }
            }
            let robustness = formula
                .robustness(&trace)
                .unwrap_or_else(|e| panic!("{name} did not evaluate: {e}"));
            assert!(
                robustness.is_finite(),
                "{name} robustness {robustness} is not finite"
            );
        }
    }

    #[test]
    fn overshoot_scores_an_excess_overshoot_negative() {
        use crate::{Formula, Trace};

        let text = SpecRegistry::default()
            .builder("controls/overshoot")
            .unwrap()
            .build_deterministic()
            .unwrap();
        let formula = Formula::parse(&text).unwrap();
        let times = vec![0.0, 10.0, 20.0, 30.0];
        let mut trace = Trace::new(times.clone()).unwrap();
        trace.add_signal("output", vec![0.5; times.len()]).unwrap();
        trace
            .add_signal("reference", vec![0.4; times.len()])
            .unwrap();
        let robustness = formula.robustness(&trace).unwrap();
        assert!((robustness + 0.05).abs() < 1e-9, "robustness {robustness}");
    }

    #[test]
    fn band_specs_score_holding_and_violating_traces() {
        use crate::{Formula, Trace};

        let cases = [
            ("medical/euglycemia_band", "glucose", 100.0, 50.0),
            ("power/voltage_band", "voltage", 1.0, 1.2),
            ("networking/latency_bound", "one_way_delay", 0.05, 0.15),
            ("robotics/velocity_limit", "velocity", 0.5, 1.5),
            ("automotive/speed_limit", "speed", 20.0, 40.0),
            ("aerospace/load_factor_limit", "load_factor", 1.0, 3.0),
            ("industrial/temperature_limit", "temperature", 25.0, 50.0),
            ("uav/altitude_band", "altitude", 50.0, 150.0),
            ("financial/max_drawdown_limit", "drawdown", 0.1, 0.3),
        ];
        let registry = SpecRegistry::default();
        let times: Vec<f64> = (0..=80).map(f64::from).collect();
        for (spec, signal, ok, bad) in cases {
            let text = registry.builder(spec).unwrap().build_deterministic().unwrap();
            let formula = Formula::parse(&text).unwrap();
            let mut holds = Trace::new(times.clone()).unwrap();
            holds.add_signal(signal, vec![ok; times.len()]).unwrap();
            assert!(
                formula.robustness(&holds).unwrap() > 0.0,
                "{spec} should hold at {ok}"
            );
            let mut fails = Trace::new(times.clone()).unwrap();
            fails.add_signal(signal, vec![bad; times.len()]).unwrap();
            assert!(
                formula.robustness(&fails).unwrap() < 0.0,
                "{spec} should fail at {bad}"
            );
        }
    }

    #[test]
    fn time_to_collision_holds_without_dividing_on_a_non_closing_trace() {
        use crate::{Formula, Trace};

        let text = SpecRegistry::default()
            .builder("automotive/time_to_collision")
            .unwrap()
            .build_deterministic()
            .unwrap();
        let formula = Formula::parse(&text).unwrap();
        let times: Vec<f64> = (0..=30).map(f64::from).collect();
        let mut trace = Trace::new(times.clone()).unwrap();
        trace.add_signal("range", vec![40.0; times.len()]).unwrap();
        trace
            .add_signal("closing_speed", vec![0.0; times.len()])
            .unwrap();
        let robustness = formula.robustness(&trace).unwrap();
        assert!(robustness > 0.0, "non-closing TTC should hold, got {robustness}");
    }
}