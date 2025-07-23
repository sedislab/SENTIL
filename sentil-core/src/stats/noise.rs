//! Noise models for stochastic signal lifting.

use rand::Rng;
use rand_distr::StandardNormal;

use crate::error::{Error, Result};

/// How a noise draw combines with a deterministic reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseInteraction {
    /// The noise is added to the reading: `reading + noise`.
    Additive,
    /// The noise scales the reading: `reading * noise`.
    Multiplicative,
}

impl NoiseInteraction {
    /// Combines a reading with a noise draw.
    pub fn apply(self, reading: f64, noise: f64) -> f64 {
        match self {
            NoiseInteraction::Additive => reading + noise,
            NoiseInteraction::Multiplicative => reading * noise,
        }
    }
}

/// A probability distribution that sensor noise is drawn from.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseModel {
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    Dirac { value: f64 },
    Gaussian { mean: f64, std_dev: f64 },
    Uniform { low: f64, high: f64 },
    LogNormal { mu: f64, sigma: f64 },
    Exponential { lambda: f64 },
    Gamma { shape: f64, scale: f64 },
    Beta { alpha: f64, beta: f64 },
    Weibull { shape: f64, scale: f64 },
    Rayleigh { scale: f64 },
    Gumbel { location: f64, scale: f64 },
    Cauchy { location: f64, scale: f64 },
    StudentT { df: f64, location: f64, scale: f64 },
}

impl NoiseModel {
    /// A point mass at `value`.
    ///
    /// # Errors
    ///
    /// Returns an error if `value` is not finite.
    pub fn dirac(value: f64) -> Result<Self> {
        finite("Dirac", "value", value)?;
        Ok(Self {
            kind: Kind::Dirac { value },
        })
    }

    /// A normal distribution.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or the standard deviation is negative.
    pub fn gaussian(mean: f64, std_dev: f64) -> Result<Self> {
        finite("Gaussian", "mean", mean)?;
        finite("Gaussian", "standard deviation", std_dev)?;
        if std_dev < 0.0 {
            return Err(invalid(
                "Gaussian",
                format!("standard deviation must be non-negative, got {std_dev}"),
            ));
        }
        Ok(Self {
            kind: Kind::Gaussian { mean, std_dev },
        })
    }

    /// A uniform distribution over `[low, high]`.
    ///
    /// # Errors
    ///
    /// Returns an error if a bound is not finite or `low` exceeds `high`.
    pub fn uniform(low: f64, high: f64) -> Result<Self> {
        finite("Uniform", "lower bound", low)?;
        finite("Uniform", "upper bound", high)?;
        if low > high {
            return Err(invalid(
                "Uniform",
                format!("lower bound {low} exceeds upper bound {high}"),
            ));
        }
        Ok(Self {
            kind: Kind::Uniform { low, high },
        })
    }

    /// A log-normal distribution with log-mean `mu` and log-standard-deviation `sigma`.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or `sigma` is negative.
    pub fn log_normal(mu: f64, sigma: f64) -> Result<Self> {
        finite("LogNormal", "log-mean", mu)?;
        finite("LogNormal", "log-standard-deviation", sigma)?;
        if sigma < 0.0 {
            return Err(invalid(
                "LogNormal",
                format!("log-standard-deviation must be non-negative, got {sigma}"),
            ));
        }
        Ok(Self {
            kind: Kind::LogNormal { mu, sigma },
        })
    }

    /// An exponential distribution with rate `lambda`.
    ///
    /// # Errors
    ///
    /// Returns an error if `lambda` is not finite or not positive.
    pub fn exponential(lambda: f64) -> Result<Self> {
        finite("Exponential", "rate", lambda)?;
        if lambda <= 0.0 {
            return Err(invalid(
                "Exponential",
                format!("rate must be positive, got {lambda}"),
            ));
        }
        Ok(Self {
            kind: Kind::Exponential { lambda },
        })
    }

    /// A gamma distribution with the given `shape` and `scale`.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or not positive.
    pub fn gamma(shape: f64, scale: f64) -> Result<Self> {
        finite("Gamma", "shape", shape)?;
        finite("Gamma", "scale", scale)?;
        if shape <= 0.0 {
            return Err(invalid(
                "Gamma",
                format!("shape must be positive, got {shape}"),
            ));
        }
        if scale <= 0.0 {
            return Err(invalid(
                "Gamma",
                format!("scale must be positive, got {scale}"),
            ));
        }
        Ok(Self {
            kind: Kind::Gamma { shape, scale },
        })
    }

    /// A beta distribution over `[0, 1]` with shape parameters `alpha` and `beta`.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or not positive.
    pub fn beta(alpha: f64, beta: f64) -> Result<Self> {
        finite("Beta", "alpha", alpha)?;
        finite("Beta", "beta", beta)?;
        if alpha <= 0.0 {
            return Err(invalid(
                "Beta",
                format!("alpha must be positive, got {alpha}"),
            ));
        }
        if beta <= 0.0 {
            return Err(invalid(
                "Beta",
                format!("beta must be positive, got {beta}"),
            ));
        }
        Ok(Self {
            kind: Kind::Beta { alpha, beta },
        })
    }

    /// A Weibull distribution.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or not positive.
    pub fn weibull(shape: f64, scale: f64) -> Result<Self> {
        positive("Weibull", "shape", shape)?;
        positive("Weibull", "scale", scale)?;
        Ok(Self {
            kind: Kind::Weibull { shape, scale },
        })
    }

    /// A Rayleigh distribution.
    ///
    /// # Errors
    ///
    /// Returns an error if `scale` is not finite or not positive.
    pub fn rayleigh(scale: f64) -> Result<Self> {
        positive("Rayleigh", "scale", scale)?;
        Ok(Self {
            kind: Kind::Rayleigh { scale },
        })
    }

    /// A Gumbel (maximum) distribution.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or `scale` is not positive.
    pub fn gumbel(location: f64, scale: f64) -> Result<Self> {
        finite("Gumbel", "location", location)?;
        positive("Gumbel", "scale", scale)?;
        Ok(Self {
            kind: Kind::Gumbel { location, scale },
        })
    }

    /// A Cauchy distribution.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or `scale` is not positive.
    pub fn cauchy(location: f64, scale: f64) -> Result<Self> {
        finite("Cauchy", "location", location)?;
        positive("Cauchy", "scale", scale)?;
        Ok(Self {
            kind: Kind::Cauchy { location, scale },
        })
    }

    /// A Student's t distribution, shifted by `location` and scaled by `scale`.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite, or `df` or `scale` is not positive.
    pub fn student_t(df: f64, location: f64, scale: f64) -> Result<Self> {
        positive("StudentT", "degrees of freedom", df)?;
        finite("StudentT", "location", location)?;
        positive("StudentT", "scale", scale)?;
        Ok(Self {
            kind: Kind::StudentT {
                df,
                location,
                scale,
            },
        })
    }

    /// Draws one value from the distribution.
    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        match self.kind {
            Kind::Dirac { value } => value,
            Kind::Gaussian { mean, std_dev } => {
                let z: f64 = rng.sample(StandardNormal);
                mean + std_dev * z
            }
            Kind::Uniform { low, high } => low + (high - low) * rng.random::<f64>(),
            Kind::LogNormal { mu, sigma } => {
                let z: f64 = rng.sample(StandardNormal);
                (mu + sigma * z).exp()
            }
            Kind::Exponential { lambda } => -(1.0 - rng.random::<f64>()).ln() / lambda,
            Kind::Gamma { shape, scale } => sample_gamma(rng, shape, scale),
            Kind::Beta { alpha, beta } => {
                let x = sample_gamma(rng, alpha, 1.0);
                let y = sample_gamma(rng, beta, 1.0);
                if x + y > 0.0 {
                    x / (x + y)
                } else {
                    0.5
                }
            }
            Kind::Weibull { shape, scale } => {
                scale * (-(1.0 - rng.random::<f64>()).ln()).powf(1.0 / shape)
            }
            Kind::Rayleigh { scale } => scale * (-2.0 * (1.0 - rng.random::<f64>()).ln()).sqrt(),
            Kind::Gumbel { location, scale } => {
                let u = rng.random::<f64>().max(f64::MIN_POSITIVE);
                location - scale * (-u.ln()).ln()
            }
            Kind::Cauchy { location, scale } => {
                let u: f64 = rng.random();
                location + scale * (std::f64::consts::PI * (u - 0.5)).tan()
            }
            Kind::StudentT {
                df,
                location,
                scale,
            } => {
                let z: f64 = rng.sample(StandardNormal);
                let chi2 = sample_gamma(rng, df / 2.0, 2.0);
                let denom = (chi2 / df).sqrt().max(f64::MIN_POSITIVE);
                location + scale * z / denom
            }
        }
    }
}

fn positive(model: &'static str, name: &str, value: f64) -> Result<()> {
    finite(model, name, value)?;
    if value <= 0.0 {
        return Err(invalid(
            model,
            format!("{name} must be positive, got {value}"),
        ));
    }
    Ok(())
}

/// A gamma variate by Marsaglia and Tsang's method.
#[allow(
    clippy::many_single_char_names,
    reason = "d, c, z, v, u are the variable names from Marsaglia and Tsang's paper"
)]
fn sample_gamma<R: Rng + ?Sized>(rng: &mut R, shape: f64, scale: f64) -> f64 {
    if shape < 1.0 {
        let u: f64 = rng.random();
        return sample_gamma(rng, shape + 1.0, scale) * u.powf(1.0 / shape);
    }
    let d = shape - 1.0 / 3.0;
    let c = 1.0 / (9.0 * d).sqrt();
    loop {
        let z: f64 = rng.sample(StandardNormal);
        let v = (1.0 + c * z).powi(3);
        if v <= 0.0 {
            continue;
        }
        let u: f64 = rng.random();
        let z2 = z * z;
        if u < 1.0 - 0.0331 * z2 * z2 || u.ln() < 0.5 * z2 + d * (1.0 - v + v.ln()) {
            return d * v * scale;
        }
    }
}

fn finite(model: &'static str, name: &str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid(
            model,
            format!("{name} must be finite, got {value}"),
        ))
    }
}

fn invalid(model: &'static str, reason: String) -> Error {
    Error::InvalidNoiseModel { model, reason }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the degenerate cases produce exact values"
    )]

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn dirac_is_deterministic() {
        let mut rng = StdRng::seed_from_u64(1);
        let model = NoiseModel::dirac(3.5).unwrap();
        assert_eq!(model.sample(&mut rng), 3.5);
        assert_eq!(model.sample(&mut rng), 3.5);
    }

    #[test]
    fn zero_variance_gaussian_returns_the_mean() {
        let mut rng = StdRng::seed_from_u64(2);
        let model = NoiseModel::gaussian(7.0, 0.0).unwrap();
        assert_eq!(model.sample(&mut rng), 7.0);
    }

    #[test]
    fn uniform_stays_within_its_bounds() {
        let mut rng = StdRng::seed_from_u64(3);
        let model = NoiseModel::uniform(-2.0, 5.0).unwrap();
        for _ in 0..1000 {
            let x = model.sample(&mut rng);
            assert!((-2.0..=5.0).contains(&x));
        }
    }

    #[test]
    fn zero_sigma_log_normal_is_a_point_mass() {
        let mut rng = StdRng::seed_from_u64(4);
        let model = NoiseModel::log_normal(1.5, 0.0).unwrap();
        assert_eq!(model.sample(&mut rng), 1.5_f64.exp());
    }

    #[test]
    fn exponential_is_positive_with_the_right_mean() {
        let mut rng = StdRng::seed_from_u64(5);
        let model = NoiseModel::exponential(4.0).unwrap();
        let n = 200_000u32;
        let mut sum = 0.0;
        for _ in 0..n {
            let x = model.sample(&mut rng);
            assert!(x > 0.0);
            sum += x;
        }
        assert!((sum / f64::from(n) - 0.25).abs() < 0.01);
    }

    fn mean_of(model: &NoiseModel, seed: u64, n: u32) -> f64 {
        let mut rng = StdRng::seed_from_u64(seed);
        (0..n).map(|_| model.sample(&mut rng)).sum::<f64>() / f64::from(n)
    }

    #[test]
    fn gamma_is_positive_with_the_right_mean() {
        let model = NoiseModel::gamma(2.0, 1.5).unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..1000 {
            assert!(model.sample(&mut rng) > 0.0);
        }
        assert!((mean_of(&model, 7, 200_000) - 3.0).abs() < 0.05);
    }

    #[test]
    fn gamma_with_small_shape_uses_the_boost_path() {
        let model = NoiseModel::gamma(0.4, 2.0).unwrap();
        assert!((mean_of(&model, 8, 300_000) - 0.8).abs() < 0.05);
    }

    #[test]
    fn beta_stays_in_the_unit_interval_with_the_right_mean() {
        let model = NoiseModel::beta(2.0, 3.0).unwrap();
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..1000 {
            assert!((0.0..=1.0).contains(&model.sample(&mut rng)));
        }
        assert!((mean_of(&model, 9, 200_000) - 0.4).abs() < 0.01);
    }

    #[test]
    fn weibull_with_unit_shape_is_exponential() {
        let model = NoiseModel::weibull(1.0, 2.0).unwrap();
        let mut rng = StdRng::seed_from_u64(10);
        for _ in 0..1000 {
            assert!(model.sample(&mut rng) >= 0.0);
        }
        assert!((mean_of(&model, 10, 200_000) - 2.0).abs() < 0.05);
    }

    #[test]
    fn rayleigh_has_the_right_mean() {
        let model = NoiseModel::rayleigh(2.0).unwrap();
        let expected = 2.0 * (std::f64::consts::FRAC_PI_2).sqrt();
        assert!((mean_of(&model, 11, 200_000) - expected).abs() < 0.05);
    }

    #[test]
    fn gumbel_has_the_right_mean() {
        let model = NoiseModel::gumbel(0.0, 1.0).unwrap();
        assert!((mean_of(&model, 12, 200_000) - 0.577_215_664_9).abs() < 0.05);
    }

    #[test]
    fn cauchy_is_centered_at_its_location() {
        let model = NoiseModel::cauchy(3.0, 2.0).unwrap();
        let mut rng = StdRng::seed_from_u64(13);
        let n = 200_000u32;
        let mut below = 0u32;
        for _ in 0..n {
            if model.sample(&mut rng) < 3.0 {
                below += 1;
            }
        }
        assert!((f64::from(below) / f64::from(n) - 0.5).abs() < 0.01);
    }

    #[test]
    fn student_t_mean_approaches_its_location() {
        let model = NoiseModel::student_t(10.0, 2.0, 1.0).unwrap();
        assert!((mean_of(&model, 14, 200_000) - 2.0).abs() < 0.05);
    }

    #[test]
    fn sampling_is_reproducible_from_a_seed() {
        let model = NoiseModel::gaussian(0.0, 1.0).unwrap();
        let mut a = StdRng::seed_from_u64(42);
        let mut b = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            assert_eq!(model.sample(&mut a), model.sample(&mut b));
        }
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        assert!(matches!(
            NoiseModel::gaussian(0.0, -1.0),
            Err(Error::InvalidNoiseModel {
                model: "Gaussian",
                ..
            })
        ));
        assert!(matches!(
            NoiseModel::uniform(5.0, 1.0),
            Err(Error::InvalidNoiseModel {
                model: "Uniform",
                ..
            })
        ));
        assert!(NoiseModel::dirac(f64::NAN).is_err());
        assert!(NoiseModel::log_normal(0.0, -1.0).is_err());
        assert!(NoiseModel::exponential(0.0).is_err());
        assert!(NoiseModel::exponential(-2.0).is_err());
        assert!(NoiseModel::gamma(0.0, 1.0).is_err());
        assert!(NoiseModel::gamma(1.0, -1.0).is_err());
        assert!(NoiseModel::beta(-1.0, 1.0).is_err());
        assert!(NoiseModel::weibull(1.0, 0.0).is_err());
        assert!(NoiseModel::rayleigh(-1.0).is_err());
        assert!(NoiseModel::gumbel(f64::NAN, 1.0).is_err());
        assert!(NoiseModel::cauchy(0.0, -1.0).is_err());
        assert!(NoiseModel::student_t(0.0, 0.0, 1.0).is_err());
    }

    #[test]
    fn interaction_combines_reading_and_noise() {
        assert_eq!(NoiseInteraction::Additive.apply(10.0, 3.0), 13.0);
        assert_eq!(NoiseInteraction::Multiplicative.apply(10.0, 3.0), 30.0);
    }
}