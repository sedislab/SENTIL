//! Stochastic models eith their closed-form probabilities

const PHI_0: f64 = 0.5;
const PHI_HALF: f64 = 0.691_462_461_274_013_1;
const PHI_1: f64 = 0.841_344_746_068_542_9;
const PHI_2: f64 = 0.977_249_868_051_820_8;
const PHI_NEG_HALF: f64 = 0.308_537_538_725_986_9;

pub struct ProbCase {
    pub id: &'static str,
    pub signals: &'static [(&'static str, &'static [f64])],
    pub noise: &'static [(&'static str, f64)],
    pub formula: &'static str,
    pub probability: f64,
    pub method: &'static str,
}

pub const PROBABILISTIC: &[ProbCase] = &[
    ProbCase { id: "boundary", signals: &[("x", &[0.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(x > 0)", probability: PHI_0, method: "normal CDF at 0" },
    ProbCase { id: "one_sigma", signals: &[("x", &[1.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(x > 0)", probability: PHI_1, method: "normal CDF at 1" },
    ProbCase { id: "two_sigma", signals: &[("x", &[2.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(x > 0)", probability: PHI_2, method: "normal CDF at 2" },
    ProbCase { id: "below_threshold", signals: &[("x", &[0.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(x > 1)", probability: 1.0 - PHI_1, method: "normal CDF at -1" },
    ProbCase { id: "wider_noise", signals: &[("x", &[1.0])], noise: &[("x", 2.0)], formula: "P >= 0.5(x > 0)", probability: PHI_HALF, method: "normal CDF at 0.5" },
    ProbCase { id: "shifted_threshold", signals: &[("x", &[5.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(x > 4)", probability: PHI_1, method: "normal CDF at 1" },
    ProbCase { id: "negative_base", signals: &[("x", &[-0.5])], noise: &[("x", 1.0)], formula: "P >= 0.5(x > 0)", probability: PHI_NEG_HALF, method: "normal CDF at -0.5" },
    ProbCase { id: "scaled_margin", signals: &[("x", &[3.0])], noise: &[("x", 1.5)], formula: "P >= 0.5(x > 0)", probability: PHI_2, method: "normal CDF at 2" },
    ProbCase { id: "always_two", signals: &[("x", &[1.0, 1.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(always[0, 1](x > 0))", probability: PHI_1 * PHI_1, method: "product of two normal CDFs at 1" },
    ProbCase { id: "always_three", signals: &[("x", &[1.0, 1.0, 1.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(always[0, 2](x > 0))", probability: PHI_1 * PHI_1 * PHI_1, method: "product of three normal CDFs at 1" },
    ProbCase { id: "always_mixed", signals: &[("x", &[2.0, 1.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(always[0, 1](x > 0))", probability: PHI_2 * PHI_1, method: "product of the normal CDFs at 2 and 1" },
    ProbCase { id: "eventually_two", signals: &[("x", &[0.0, 0.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(eventually[0, 1](x > 0))", probability: 1.0 - (1.0 - PHI_0) * (1.0 - PHI_0), method: "complement of two failure probabilities at 0" },
    ProbCase { id: "eventually_three", signals: &[("x", &[-1.0, -1.0, -1.0])], noise: &[("x", 1.0)], formula: "P >= 0.5(eventually[0, 2](x > 0))", probability: 1.0 - PHI_1 * PHI_1 * PHI_1, method: "complement of the product of three failure probabilities at -1" },
    ProbCase { id: "conjunction", signals: &[("x", &[1.0]), ("y", &[1.0])], noise: &[("x", 1.0), ("y", 1.0)], formula: "P >= 0.5((x > 0) and (y > 0))", probability: PHI_1 * PHI_1, method: "product of the two signals' normal CDFs at 1" },
];

#[cfg(test)]
mod tests {
    use super::PROBABILISTIC;
    use sentil::{Formula, LiftingRegistry, NoiseInteraction, NoiseModel, SmcConfig, Trace};

    #[test]
    fn sentil_estimates_match_the_known_probabilities() {
        for case in PROBABILISTIC {
            let phi = Formula::parse(case.formula).unwrap_or_else(|e| panic!("{}: {e}", case.id));
            let n = case.signals[0].1.len();
            let mut trace = Trace::indexed(n);
            for (name, values) in case.signals {
                trace
                    .add_signal(name, values.to_vec())
                    .expect("base signal matches the grid");
            }
            let mut lifting = LiftingRegistry::new();
            for (var, std_dev) in case.noise {
                lifting.register(
                    var,
                    NoiseModel::gaussian(0.0, *std_dev).expect("a valid deviation"),
                    NoiseInteraction::Additive,
                );
            }
            let config = SmcConfig {
                samples: 50_000,
                confidence: 0.95,
                seed: 7,
                ..Default::default()
            };
            let result = phi.check(&trace, &lifting, &config).expect("a probability");
            assert!(
                (result.probability - case.probability).abs() < 0.01,
                "{}: estimate {} vs known {} ({})",
                case.id,
                result.probability,
                case.probability,
                case.method
            );
        }
    }
}