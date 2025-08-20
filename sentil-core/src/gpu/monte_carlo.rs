//! GPU Monte Carlo for the atemporal statistical model checking case.
//!
//! For a probabilistic formula `P~p(phi)` whose inner `phi` is atemporal, the
//! satisfaction probability is estimated by drawing many noisy realizations of
//! the reading and counting how many satisfy `phi`. This module runs that count
//! on a GPU: each thread draws one realization, evaluates the transpiled
//! formula, and a tree reduction counts the satisfied ones.
//!
//! The GPU works in f32, so its estimate agrees with the CPU only within Monte
//! Carlo and single-precision tolerance. The device returns only an integer
//! count, and the confidence interval and the verdict are computed on the host
//! in f64, identical to the CPU path. The path runs only for the closed-form
//! noise families and falls back to the CPU for everything else, so a result is
//! always available.

// The pieces are built bottom-up and become reachable from the statistical layer
// once the SMC entry wires in the fallback path.
#![allow(dead_code)]

use core::fmt::Write as _;

use pollster::FutureExt as _;

use super::transpiler::{transpile_atemporal, validate};
use crate::error::Error;
use crate::formula::Formula;
use crate::stats::{GpuSampler, LiftingRegistry, NoiseInteraction};

/// The width, in f32 slots, of one variable's noise record in the device buffer.
pub(crate) const NOISE_RECORD: usize = 8;

/// A failure on the GPU Monte Carlo path.
///
/// A capability or policy miss (no device, an unsupported family, too many
/// samples) is handled by falling back to the CPU, not by surfacing one of
/// these. A variant that does reach the caller becomes [`Error::Gpu`].
#[derive(Debug)]
pub(crate) enum GpuMcError {
    AdapterNotFound,
    DeviceRequest(String),
    Readback(String),
    InvalidWgsl(String),
    /// More samples were requested than the f32-exact count path allows. The
    /// CPU path, which counts in `u64`, handles larger runs.
    SampleCountOverflow {
        /// The requested sample count.
        requested: u64,
        /// The largest count the GPU path accepts.
        max: u64,
    },
    /// A noise family has no closed-form GPU sampler. The caller runs on the CPU.
    UnsupportedNoiseFamily {
        /// The family that has no GPU sampler.
        family: &'static str,
    },
}

impl core::fmt::Display for GpuMcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GpuMcError::AdapterNotFound => write!(f, "no compatible GPU adapter is present"),
            GpuMcError::DeviceRequest(e) => write!(f, "could not create a GPU device: {e}"),
            GpuMcError::Readback(e) => write!(f, "could not read GPU results back: {e}"),
            GpuMcError::InvalidWgsl(e) => write!(f, "the GPU shader did not compile: {e}"),
            GpuMcError::SampleCountOverflow { requested, max } => write!(
                f,
                "{requested} samples exceeds the GPU limit of {max}; the CPU path handles larger runs"
            ),
            GpuMcError::UnsupportedNoiseFamily { family } => write!(
                f,
                "the {family} noise family has no GPU sampler; this runs on the CPU"
            ),
        }
    }
}

impl From<GpuMcError> for Error {
    fn from(error: GpuMcError) -> Self {
        Error::Gpu {
            message: error.to_string(),
        }
    }
}

/// Packs each variable's noise parameters into the device buffer, in `symbols` order.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the device runs in f32; the small family tag and the parameters fit it"
)]
pub(crate) fn pack_noise_params(
    symbols: &[String],
    lifting: &LiftingRegistry,
) -> Result<Vec<f32>, GpuMcError> {
    let mut packed = vec![0.0f32; symbols.len() * NOISE_RECORD];
    for (slot, name) in symbols.iter().enumerate() {
        let Some((model, interaction)) = lifting.model_for(name) else {
            continue;
        };
        let (family, p0, p1) = match model.gpu_sampler() {
            GpuSampler::Closed { family, p0, p1 } => (family, p0, p1),
            GpuSampler::Cpu { family } => {
                return Err(GpuMcError::UnsupportedNoiseFamily { family })
            }
        };
        let base = slot * NOISE_RECORD;
        packed[base] = family as f32;
        packed[base + 1] = match interaction {
            NoiseInteraction::Additive => 0.0,
            NoiseInteraction::Multiplicative => 1.0,
        };
        packed[base + 2] = p0 as f32;
        packed[base + 3] = p1 as f32;
    }
    Ok(packed)
}

const SHADER_PRELUDE: &str = r"struct Params {
    n_samples: u32,
    seed: u32,
    threshold: f32,
    num_workgroups: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> results: array<f32>;
@group(0) @binding(2) var<storage, read_write> reduction_buffer: array<f32>;
@group(0) @binding(3) var<storage, read> base_state: array<f32>;
@group(0) @binding(4) var<storage, read> noise_params: array<f32>;

fn mul_hi(a: u32, b: u32) -> u32 {
    let a_lo = a & 0xFFFFu;
    let a_hi = a >> 16u;
    let b_lo = b & 0xFFFFu;
    let b_hi = b >> 16u;
    let cross = (a_lo * b_lo >> 16u) + (a_hi * b_lo & 0xFFFFu) + a_lo * b_hi;
    return a_hi * b_hi + (a_hi * b_lo >> 16u) + (cross >> 16u);
}

fn philox2x32_round(ctr: vec2<u32>, key: u32) -> vec2<u32> {
    let hi = mul_hi(ctr.x, 0xD256D193u);
    let lo = ctr.x * 0xD256D193u;
    return vec2<u32>(hi ^ key ^ ctr.y, lo);
}

fn philox2x32(ctr: vec2<u32>, key: u32) -> vec2<u32> {
    var c = ctr;
    var k = key;
    for (var i = 0u; i < 10u; i = i + 1u) {
        c = philox2x32_round(c, k);
        k = k + 0x9E3779B9u;
    }
    return c;
}

fn init_rng(global_id: u32, seed: u32) -> u32 {
    let result = philox2x32(vec2<u32>(global_id, seed), seed ^ 0xDEADBEEFu);
    return result.x ^ result.y;
}

fn xorshift32(state: ptr<function, u32>) -> u32 {
    var x = *state;
    x = x ^ (x << 13u);
    x = x ^ (x >> 17u);
    x = x ^ (x << 5u);
    *state = x;
    return x;
}

fn rand_f32(state: ptr<function, u32>) -> f32 {
    return f32(xorshift32(state)) * (1.0 / 4294967296.0);
}

fn rand_normal(state: ptr<function, u32>) -> f32 {
    let u1 = max(rand_f32(state), 1e-10);
    let u2 = rand_f32(state);
    return sqrt(-2.0 * log(u1)) * cos(6.283185307179586 * u2);
}
";

/// One partial count per workgroup, summed on the host.
const SHADER_REDUCE: &str = r"
var<workgroup> shared_count: array<u32, 256>;

@compute @workgroup_size(256)
fn reduce_count_kernel(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>
) {
    let tid = local_id.x;
    let gid = global_id.x;

    var local_count = 0u;
    if (gid < params.n_samples) {
        if (results[gid] >= params.threshold) {
            local_count = 1u;
        }
    }

    shared_count[tid] = local_count;
    workgroupBarrier();

    for (var s = 128u; s > 0u; s = s >> 1u) {
        if (tid < s) {
            shared_count[tid] = shared_count[tid] + shared_count[tid + s];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        reduction_buffer[group_id.x] = f32(shared_count[0]);
    }
}";

/// Assembles the full count shader for `formula`: the prelude, the transpiled
/// `evaluate_formula`, the per-sample kernel, and the reduction.
///
/// Returns the shader source and the number of variable slots the kernel reads.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when the inner formula is not atemporal or the shader does not validate.
pub(crate) fn build_count_shader(
    formula: &Formula,
    symbols: &[String],
) -> Result<(String, usize), Error> {
    let transpiled = transpile_atemporal(formula, symbols)?;
    let array_size = transpiled.state_size.max(1);
    let mut source = String::from(SHADER_PRELUDE);
    source.push('\n');
    source.push_str(&transpiled.evaluate_formula);
    write_simulation_kernel(&mut source, array_size, transpiled.state_size);
    source.push_str(SHADER_REDUCE);
    validate(&source)?;
    Ok((source, transpiled.state_size))
}

/// Writes the per-sample kernel: draw a residual for each modeled slot, fold it
/// into the base reading by the slot's interaction, and score the formula. The
/// family arms cover the closed-form samplers only; an unmodeled slot is read
/// straight through without a draw, so there is no identity arm in `draw_residual`.
fn write_simulation_kernel(source: &mut String, array_size: usize, state_size: usize) {
    let _ = write!(
        source,
        r"
fn draw_residual(slot: u32, rng: ptr<function, u32>) -> f32 {{
    let b = 8u * slot;
    let p0 = noise_params[b + 2u];
    let p1 = noise_params[b + 3u];
    let family = noise_params[b];
    if (family < 1.5) {{
        return p0 + p1 * rand_normal(rng);
    }} else if (family < 2.5) {{
        return exp(p0 + p1 * rand_normal(rng));
    }} else if (family < 3.5) {{
        return -log(max(1.0 - rand_f32(rng), 1e-38)) / p0;
    }} else if (family < 4.5) {{
        return p0 + (p1 - p0) * rand_f32(rng);
    }} else {{
        return p0;
    }}
}}

@compute @workgroup_size(256)
fn simulation_kernel(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let i = global_id.x;
    if (i >= params.n_samples) {{
        return;
    }}
    var rng = init_rng(i, params.seed);
    var state: array<f32, {array_size}>;
    for (var s = 0u; s < {state_size}u; s = s + 1u) {{
        let base = base_state[s];
        if (noise_params[8u * s] < 0.5) {{
            state[s] = base;
        }} else {{
            let r = draw_residual(s, &rng);
            if (noise_params[8u * s + 1u] < 0.5) {{
                state[s] = base + r;
            }} else {{
                state[s] = base * r;
            }}
        }}
    }}
    results[i] = evaluate_formula(state);
}}
"
    );
}

/// The uniform block the kernels read, matching the WGSL `Params`: four 4-byte
/// fields, 16 bytes. `threshold` is always `0.0`, since the count is of
/// realizations whose robustness is at or above zero.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    n_samples: u32,
    seed: u32,
    threshold: f32,
    num_workgroups: u32,
}

/// A device and the two count pipelines.
pub(crate) struct GpuMcContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    simulation_pipeline: wgpu::ComputePipeline,
    reduce_count_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuMcContext {
    /// Builds a context from the assembled count shader; `temporal` adds the time-grid binding.
    ///
    /// # Errors
    ///
    /// Returns [`GpuMcError::AdapterNotFound`] when no adapter is present,
    /// [`GpuMcError::DeviceRequest`] when the device cannot be created, and
    /// [`GpuMcError::InvalidWgsl`] when the shader does not compile.
    pub(crate) fn new(shader_source: &str) -> Result<Self, GpuMcError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
            .map_err(|_| GpuMcError::AdapterNotFound)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sentil gpu monte carlo"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::default(),
            })
            .block_on()
            .map_err(|e| GpuMcError::DeviceRequest(e.to_string()))?;

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sentil count shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let storage = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sentil count layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(1, false),
                storage(2, false),
                storage(3, true),
                storage(4, true),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sentil count pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = |label: &str, entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let simulation_pipeline = pipeline("sentil simulation", "simulation_kernel");
        let reduce_count_pipeline = pipeline("sentil reduce count", "reduce_count_kernel");

        if let Some(err) = device.pop_error_scope().block_on() {
            return Err(GpuMcError::InvalidWgsl(err.to_string()));
        }
        Ok(Self {
            device,
            queue,
            simulation_pipeline,
            reduce_count_pipeline,
            bind_group_layout,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the packed records are exact f32 values")]

    use super::*;
    use crate::stats::NoiseModel;

    #[test]
    fn packs_supported_families_by_slot() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 2.0).unwrap(),
            NoiseInteraction::Additive,
        );
        lifting.register(
            "y",
            NoiseModel::uniform(1.0, 3.0).unwrap(),
            NoiseInteraction::Multiplicative,
        );
        let symbols = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let packed = pack_noise_params(&symbols, &lifting).unwrap();
        assert_eq!(packed.len(), 3 * NOISE_RECORD);
        // x: Gaussian (1), additive (0), mean 0, std 2.
        assert_eq!(&packed[0..4], &[1.0f32, 0.0, 0.0, 2.0]);
        // y: Uniform (4), multiplicative (1), low 1, high 3.
        assert_eq!(&packed[8..12], &[4.0f32, 1.0, 1.0, 3.0]);
        assert_eq!(&packed[16..24], &[0.0f32; 8]);
    }

    #[test]
    fn an_unsupported_family_declines_for_cpu_fallback() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gamma(2.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let err = pack_noise_params(&["x".to_string()], &lifting).unwrap_err();
        assert!(matches!(
            err,
            GpuMcError::UnsupportedNoiseFamily { family: "Gamma" }
        ));
    }

    #[test]
    fn count_shader_assembles_and_validates() {
        let formula = Formula::parse("x > 5 and y < 3").unwrap();
        let symbols = formula.variables();
        let (source, state_size) = build_count_shader(&formula, &symbols).unwrap();
        assert_eq!(state_size, 2);
        assert!(source.contains("fn simulation_kernel"));
        assert!(source.contains("fn evaluate_formula"));
        assert!(source.contains("fn reduce_count_kernel"));
        assert!(source.contains("fn draw_residual"));
    }

    #[test]
    fn a_temporal_formula_has_no_count_shader() {
        let formula = Formula::parse("always[0, 3](x > 0)").unwrap();
        let symbols = formula.variables();
        assert!(build_count_shader(&formula, &symbols).is_err());
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn context_builds_on_a_device() {
        let formula = Formula::parse("x > 0").unwrap();
        let symbols = formula.variables();
        let (shader, _) = build_count_shader(&formula, &symbols).unwrap();
        assert!(GpuMcContext::new(&shader).is_ok());
    }
}