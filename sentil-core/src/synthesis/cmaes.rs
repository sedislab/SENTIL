//! Covariance matrix adaptation evolution strategy (CMA-ES), in the standard
//! `(mu/mu_w, lambda)` formulation.

use super::model::Bounds;
use super::numerics::symmetric_eigen;
use crate::error::Result;

/// A SplitMix64 generator with standard-normal sampling.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the top 53 bits are exact in f64"
    )]
    fn standard_normal(&mut self) -> f64 {
        let unit = |bits: u64| (bits >> 11) as f64 / (1u64 << 53) as f64;
        let u1 = unit(self.next_u64()).max(f64::MIN_POSITIVE);
        let u2 = unit(self.next_u64());
        (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
    }
}

/// Tuning for [`cma_es`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmaConfig {
    /// The population per generation; `0` chooses the default from the dimension.
    pub population: usize,
    /// The maximum number of generations.
    pub max_generations: usize,
    /// The initial step size.
    pub initial_step: f64,
    /// The seed.
    pub seed: u64,
}

impl Default for CmaConfig {
    fn default() -> Self {
        Self {
            population: 0,
            max_generations: 300,
            initial_step: 0.3,
            seed: 42,
        }
    }
}

/// Maximizes a black-box `objective` over the box `bounds`, starting from `start`,
/// by CMA-ES. Returns the best point and value found; the objective is opaque, so
/// no gradient is used.
///
/// # Errors
///
/// Propagates any error the objective returns, and a non-square covariance from
/// the eigensolver (which cannot arise here).
#[allow(
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "CMA-ES uses standard short notation and small exact integer-to-float casts"
)]
pub fn cma_es<F>(
    objective: F,
    start: &[f64],
    bounds: &Bounds,
    config: CmaConfig,
) -> Result<(Vec<f64>, f64)>
where
    F: Fn(&[f64]) -> Result<f64>,
{
    let n = start.len();
    let nf = n as f64;
    let lambda = if config.population == 0 {
        4 + (3.0 * nf.max(1.0).ln()).floor() as usize
    } else {
        config.population
    };
    let mu = (lambda / 2).max(1);

    let mut weights: Vec<f64> = (1..=mu)
        .map(|i| (mu as f64 + 0.5).ln() - (i as f64).ln())
        .collect();
    let weight_sum: f64 = weights.iter().sum();
    for w in &mut weights {
        *w /= weight_sum;
    }
    let mu_eff = 1.0 / weights.iter().map(|w| w * w).sum::<f64>();

    let c_sigma = (mu_eff + 2.0) / (nf + mu_eff + 5.0);
    let d_sigma = 1.0 + 2.0 * (((mu_eff - 1.0) / (nf + 1.0)).sqrt() - 1.0).max(0.0) + c_sigma;
    let c_c = (4.0 + mu_eff / nf) / (nf + 4.0 + 2.0 * mu_eff / nf);
    let c_1 = 2.0 / ((nf + 1.3) * (nf + 1.3) + mu_eff);
    let c_mu =
        (1.0 - c_1).min(2.0 * (mu_eff - 2.0 + 1.0 / mu_eff) / ((nf + 2.0) * (nf + 2.0) + mu_eff));
    let chi_n = nf.sqrt() * (1.0 - 1.0 / (4.0 * nf) + 1.0 / (21.0 * nf * nf));

    let mut rng = Rng::new(config.seed);
    let mut mean = start.to_vec();
    bounds.clamp(&mut mean);
    let mut sigma = config.initial_step;
    let mut cov: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| f64::from(u8::from(i == j))).collect())
        .collect();
    let mut path_sigma = vec![0.0; n];
    let mut path_c = vec![0.0; n];
    let mut best = mean.clone();
    let mut best_value = objective(&best)?;

    for generation in 0..config.max_generations {
        let (eigenvalues, eigenvectors) = symmetric_eigen(&cov)?;
        let roots: Vec<f64> = eigenvalues.iter().map(|&l| l.max(0.0).sqrt()).collect();
        let inverse_roots: Vec<f64> = roots
            .iter()
            .map(|&d| if d > 0.0 { 1.0 / d } else { 0.0 })
            .collect();

        let mut offspring: Vec<(f64, Vec<f64>)> = Vec::with_capacity(lambda);
        for _ in 0..lambda {
            let z: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
            let y = transform(&eigenvectors, &roots, &z);
            let mut point: Vec<f64> = mean
                .iter()
                .zip(&y)
                .map(|(&m, &yi)| sigma.mul_add(yi, m))
                .collect();
            bounds.clamp(&mut point);
            let value = objective(&point)?;
            if value > best_value {
                best_value = value;
                best.clone_from(&point);
            }
            offspring.push((value, y));
        }
        offspring.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut step = vec![0.0; n];
        for (&w, (_, y)) in weights.iter().zip(&offspring) {
            for (acc, &yi) in step.iter_mut().zip(y) {
                *acc += w * yi;
            }
        }
        for (m, &yi) in mean.iter_mut().zip(&step) {
            *m += sigma * yi;
        }

        let normalized = transform(&eigenvectors, &inverse_roots, &step);
        let ps_gain = (c_sigma * (2.0 - c_sigma) * mu_eff).sqrt();
        for (ps, &ni) in path_sigma.iter_mut().zip(&normalized) {
            *ps = (1.0 - c_sigma) * *ps + ps_gain * ni;
        }
        let ps_norm = path_sigma.iter().map(|x| x * x).sum::<f64>().sqrt();
        sigma *= ((c_sigma / d_sigma) * (ps_norm / chi_n - 1.0)).exp();

        let decorrelated =
            ps_norm / (1.0 - (1.0 - c_sigma).powf(2.0 * (generation as f64 + 1.0))).sqrt();
        let h_sigma = f64::from(u8::from(decorrelated < (1.4 + 2.0 / (nf + 1.0)) * chi_n));
        let pc_gain = h_sigma * (c_c * (2.0 - c_c) * mu_eff).sqrt();
        for (pc, &yi) in path_c.iter_mut().zip(&step) {
            *pc = (1.0 - c_c) * *pc + pc_gain * yi;
        }

        let decay = 1.0 - c_1 - c_mu + c_1 * (1.0 - h_sigma) * c_c * (2.0 - c_c);
        for i in 0..n {
            for j in 0..n {
                let rank_mu: f64 = weights
                    .iter()
                    .zip(&offspring)
                    .map(|(&w, (_, y))| w * y[i] * y[j])
                    .sum();
                cov[i][j] = decay * cov[i][j] + c_1 * path_c[i] * path_c[j] + c_mu * rank_mu;
            }
        }
    }
    Ok((best, best_value))
}

/// `sum_j scales[j] * dot(eigenvectors[j], vector) * eigenvectors[j]`, the action
/// of a matrix `B diag(scales) Bᵀ` on `vector` given its eigenvectors.
fn transform(eigenvectors: &[Vec<f64>], scales: &[f64], vector: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; vector.len()];
    for (basis, &scale) in eigenvectors.iter().zip(scales) {
        let projection = scale * basis.iter().zip(vector).map(|(b, v)| b * v).sum::<f64>();
        for (o, &component) in out.iter_mut().zip(basis) {
            *o += projection * component;
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_the_sphere_optimum() {
        let objective = |x: &[f64]| Ok(-x.iter().map(|v| v * v).sum::<f64>());
        let bounds = Bounds::new(vec![-5.0; 3], vec![5.0; 3]).unwrap();
        let (best, value) =
            cma_es(objective, &[3.0, -2.0, 4.0], &bounds, CmaConfig::default()).unwrap();
        assert!(value > -1e-6);
        for coordinate in &best {
            assert!(coordinate.abs() < 1e-3);
        }
    }

    #[test]
    fn solves_the_rosenbrock_valley() {
        let objective = |x: &[f64]| {
            let (a, b) = (1.0 - x[0], x[1] - x[0] * x[0]);
            Ok(-a.mul_add(a, 100.0 * b * b))
        };
        let bounds = Bounds::new(vec![-5.0, -5.0], vec![5.0, 5.0]).unwrap();
        let config = CmaConfig {
            max_generations: 500,
            ..CmaConfig::default()
        };
        let (best, value) = cma_es(objective, &[-1.0, 2.0], &bounds, config).unwrap();
        assert!(value > -1e-3, "value {value}");
        assert!((best[0] - 1.0).abs() < 0.05 && (best[1] - 1.0).abs() < 0.05);
    }
}