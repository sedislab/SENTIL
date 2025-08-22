//! GPU Monte Carlo for the atemporal statistical model checking case.

use core::fmt::Write as _;

use pollster::FutureExt as _;
use wgpu::util::DeviceExt as _;

use super::temporal::transpile_temporal;
use super::transpiler::{transpile_atemporal, validate};
use crate::error::Error;
use crate::formula::Formula;
use crate::stats::{GpuSampler, LiftingRegistry, NoiseInteraction};

/// The width, in f32 slots, of one variable's noise record in the device buffer.
pub(crate) const NOISE_RECORD: usize = 8;

/// A failure on the GPU Monte Carlo path.
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
        let (family, params) = match model.gpu_sampler() {
            GpuSampler::Device { family, params } => (family, params),
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
        for (i, &p) in params.iter().enumerate() {
            packed[base + 2 + i] = p as f32;
        }
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
";

/// The counter-based PRNG the GPU kernels share: a thread's draws depend only on
/// the index and seed it starts from, so the Monte Carlo, temporal, and rare-event
/// splitting shaders all draw the same way from their own bindings.
pub(crate) const PRNG_WGSL: &str = r"fn mul_hi(a: u32, b: u32) -> u32 {
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
    let state = result.x ^ result.y;
    return select(state, 0x1u, state == 0u);
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

// Marsaglia and Tsang's gamma variate.
fn sample_gamma(shape: f32, scale: f32, rng: ptr<function, u32>) -> f32 {
    var s = shape;
    var boost = 1.0;
    if (s < 1.0) {
        boost = pow(max(rand_f32(rng), 1e-38), 1.0 / s);
        s = s + 1.0;
    }
    let d = s - 1.0 / 3.0;
    let c = 1.0 / sqrt(9.0 * d);
    for (var i = 0u; i < 256u; i = i + 1u) {
        let z = rand_normal(rng);
        let base = 1.0 + c * z;
        let v = base * base * base;
        if (v <= 0.0) {
            continue;
        }
        let u = rand_f32(rng);
        let z2 = z * z;
        if (u < 1.0 - 0.0331 * z2 * z2 || log(max(u, 1e-38)) < 0.5 * z2 + d * (1.0 - v + log(v))) {
            return d * v * scale * boost;
        }
    }
    return s * scale * boost;
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

/// Assembles the count shader for `formula`.
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
    source.push_str(PRNG_WGSL);
    source.push('\n');
    source.push_str(&transpiled.evaluate_formula);
    write_simulation_kernel(&mut source, array_size, transpiled.state_size);
    source.push_str(SHADER_REDUCE);
    validate(&source)?;
    Ok((source, transpiled.state_size))
}

/// Writes `draw_residual`, the per-family sampler the kernel calls for each modeled slot.
pub(crate) fn write_draw_residual(source: &mut String) {
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
    }} else if (family < 5.5) {{
        return p0;
    }} else if (family < 6.5) {{
        return p1 * pow(-log(max(1.0 - rand_f32(rng), 1e-38)), 1.0 / p0);
    }} else if (family < 7.5) {{
        return p0 * sqrt(-2.0 * log(max(1.0 - rand_f32(rng), 1e-38)));
    }} else if (family < 8.5) {{
        return p0 - p1 * log(-log(max(rand_f32(rng), 1e-38)));
    }} else if (family < 9.5) {{
        return p0 + p1 * tan(3.141592653589793 * (rand_f32(rng) - 0.5));
    }} else if (family < 10.5) {{
        let lo = noise_params[b + 4u];
        let hi = noise_params[b + 5u];
        var out = clamp(p0, lo, hi);
        for (var k = 0u; k < 256u; k = k + 1u) {{
            let x = p0 + p1 * rand_normal(rng);
            if (x >= lo && x <= hi) {{
                out = x;
                break;
            }}
        }}
        return out;
    }} else if (family < 11.5) {{
        return sample_gamma(p0, p1, rng);
    }} else if (family < 12.5) {{
        let gx = sample_gamma(p0, 1.0, rng);
        let gy = sample_gamma(p1, 1.0, rng);
        if (gx + gy > 0.0) {{
            return gx / (gx + gy);
        }}
        return 0.5;
    }} else if (family < 13.5) {{
        let z = rand_normal(rng);
        let chi2 = sample_gamma(p0 * 0.5, 2.0, rng);
        let denom = max(sqrt(chi2 / p0), 1e-38);
        return p1 + noise_params[b + 4u] * z / denom;
    }} else if (family < 14.5) {{
        if (p0 < 30.0) {{
            let threshold = exp(-p0);
            var k = 0.0;
            var product = 1.0;
            for (var i = 0u; i < 1024u; i = i + 1u) {{
                k = k + 1.0;
                product = product * rand_f32(rng);
                if (product <= threshold) {{
                    break;
                }}
            }}
            return k - 1.0;
        }}
        return max(round(p0 + sqrt(p0) * rand_normal(rng)), 0.0);
    }} else {{
        let trials = u32(p0);
        var count = 0.0;
        for (var i = 0u; i < trials; i = i + 1u) {{
            if (rand_f32(rng) < p1) {{
                count = count + 1.0;
            }}
        }}
        return count;
    }}
}}
"
    );
}

/// Writes the atemporal per-sample kernel.
fn write_simulation_kernel(source: &mut String, array_size: usize, state_size: usize) {
    write_draw_residual(source);
    let _ = write!(
        source,
        r"
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

/// Writes the temporal per-sample kernel.
fn write_temporal_simulation_kernel(source: &mut String, state_size: usize, trace_len: usize) {
    write_draw_residual(source);
    let v = state_size.max(1);
    let _ = write!(
        source,
        r"
@compute @workgroup_size(256)
fn simulation_kernel(@builtin(global_invocation_id) global_id: vec3<u32>) {{
    let t = global_id.x;
    if (t >= params.n_samples) {{
        return;
    }}
    var rng = init_rng(t, params.seed);
    var trace: array<array<f32, {trace_len}>, {v}>;
    var times: array<f32, {trace_len}>;
    for (var i = 0u; i < {trace_len}u; i = i + 1u) {{
        times[i] = times_buf[i];
        for (var s = 0u; s < {v}u; s = s + 1u) {{
            let base = base_state[s * {trace_len}u + i];
            if (noise_params[8u * s] < 0.5) {{
                trace[s][i] = base;
            }} else {{
                let r = draw_residual(s, &rng);
                if (noise_params[8u * s + 1u] < 0.5) {{
                    trace[s][i] = base + r;
                }} else {{
                    trace[s][i] = base * r;
                }}
            }}
        }}
    }}
    results[t] = evaluate_temporal(&trace, &times);
}}
"
    );
}

/// Assembles the count shader for a temporal `formula`, which adds a `times` binding.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when the formula cannot be lowered or the shader does not validate.
pub(crate) fn build_temporal_shader(
    formula: &Formula,
    symbols: &[String],
    trace_len: usize,
) -> Result<(String, usize), Error> {
    let shader = transpile_temporal(formula, symbols, trace_len)?;
    let mut source = String::from(SHADER_PRELUDE);
    source.push_str(PRNG_WGSL);
    source.push_str("\n@group(0) @binding(5) var<storage, read> times_buf: array<f32>;\n\n");
    source.push_str(&shader.evaluate_temporal);
    write_temporal_simulation_kernel(&mut source, shader.state_size, shader.trace_len);
    source.push_str(SHADER_REDUCE);
    validate(&source)?;
    Ok((source, shader.state_size))
}

/// Threads per workgroup for both kernels.
const WORKGROUP_SIZE: u32 = 256;

/// WebGPU caps one dispatch dimension at 65535 workgroups.
const MAX_DISPATCH_PER_DIM: u32 = 65535;

/// The most samples one dispatch covers, one thread per sample.
const MAX_GPU_SAMPLES: u64 = MAX_DISPATCH_PER_DIM as u64 * WORKGROUP_SIZE as u64;

/// The largest integer f32 represents exactly. The partial counts return as f32,
/// so a larger total would not round-trip; the CPU path, counting in u64, takes
/// over past here.
const MAX_F32_EXACT_INT: u64 = 1 << 24;

/// The count-path sample cap: the smaller of the dispatch limit and the f32-exact
/// limit, so both the dispatch and the partial-sum readback stay exact.
const MAX_COUNT_SAMPLES: u64 = if MAX_GPU_SAMPLES < MAX_F32_EXACT_INT {
    MAX_GPU_SAMPLES
} else {
    MAX_F32_EXACT_INT
};

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
    /// Returns [`GpuMcError::AdapterNotFound`], [`GpuMcError::DeviceRequest`], or [`GpuMcError::InvalidWgsl`].
    pub(crate) fn new(shader_source: &str, temporal: bool) -> Result<Self, GpuMcError> {
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
        let mut entries = vec![
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
        ];
        if temporal {
            entries.push(storage(5, true));
        }
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sentil count layout"),
            entries: &entries,
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

    /// Counts how many of `n` noisy realizations satisfy the formula.
    ///
    /// Each thread reads the deterministic reading from `base_state`, draws a
    /// residual per modeled variable from `noise_params`, scores the formula, and
    /// the reduction tallies the realizations with robustness `>= 0`. The
    /// per-workgroup partials come back as f32 and are summed on the host.
    ///
    /// `base_state` and `noise_params` must be non-empty; the caller routes the
    /// no-variable case to the CPU before reaching here.
    ///
    /// # Errors
    ///
    /// Returns [`GpuMcError::SampleCountOverflow`] when `n` is past the cap, and
    /// [`GpuMcError::Readback`] when mapping or polling the result fails.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "n is capped below 2^24, so it fits u32"
    )]
    pub(crate) fn gpu_satisfaction_count(
        &self,
        base_state: &[f32],
        noise_params: &[f32],
        times: Option<&[f32]>,
        n: u64,
        seed: u32,
    ) -> Result<u64, GpuMcError> {
        if n == 0 {
            return Ok(0);
        }
        if n > MAX_COUNT_SAMPLES {
            return Err(GpuMcError::SampleCountOverflow {
                requested: n,
                max: MAX_COUNT_SAMPLES,
            });
        }
        let n32 = n as u32;
        let num_workgroups = n32.div_ceil(WORKGROUP_SIZE);
        let params = Params {
            n_samples: n32,
            seed,
            threshold: 0.0,
            num_workgroups,
        };

        let (submission, readback) = self.dispatch_count(&params, base_state, noise_params, times);
        self.read_partial_counts(&readback, submission, num_workgroups as usize)
    }

    fn dispatch_count(
        &self,
        params: &Params,
        base_state: &[f32],
        noise_params: &[f32],
        times: Option<&[f32]>,
    ) -> (wgpu::SubmissionIndex, wgpu::Buffer) {
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("params"),
                contents: bytemuck::bytes_of(params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let storage_init = |label: &str, contents: &[u8]| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };
        let base_buffer = storage_init("base state", bytemuck::cast_slice(base_state));
        let noise_buffer = storage_init("noise params", bytemuck::cast_slice(noise_params));
        let times_buffer = times.map(|grid| storage_init("times", bytemuck::cast_slice(grid)));
        let results_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("results"),
            size: u64::from(params.n_samples) * 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let reduction_bytes = u64::from(params.num_workgroups) * 4;
        let reduction_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reduction"),
            size: reduction_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: reduction_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut bind_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: results_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: reduction_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: base_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: noise_buffer.as_entire_binding(),
            },
        ];
        if let Some(times_buffer) = &times_buffer {
            bind_entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: times_buffer.as_entire_binding(),
            });
        }
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("count bind group"),
            layout: &self.bind_group_layout,
            entries: &bind_entries,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("count encoder"),
            });
        for (label, pipeline) in [
            ("simulation", &self.simulation_pipeline),
            ("reduce count", &self.reduce_count_pipeline),
        ] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(params.num_workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&reduction_buffer, 0, &readback_buffer, 0, reduction_bytes);
        let submission = self.queue.submit(Some(encoder.finish()));
        (submission, readback_buffer)
    }

    /// Sums the per-workgroup f32 partial counts.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "each f32 partial is an exact integer count below 2^24"
    )]
    fn read_partial_counts(
        &self,
        readback: &wgpu::Buffer,
        submission: wgpu::SubmissionIndex,
        num_workgroups: usize,
    ) -> Result<u64, GpuMcError> {
        let slice = readback.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| GpuMcError::Readback(e.to_string()))?;
        match rx.block_on() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(GpuMcError::Readback(e.to_string())),
            Err(e) => return Err(GpuMcError::Readback(e.to_string())),
        }

        let data = slice.get_mapped_range();
        let mut total = 0u64;
        for chunk in data.chunks_exact(4).take(num_workgroups) {
            let bytes: [u8; 4] = chunk
                .try_into()
                .map_err(|_| GpuMcError::Readback("a partial count was not four bytes".into()))?;
            total += f32::from_le_bytes(bytes) as u64;
        }
        drop(data);
        readback.unmap();
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the packed records are exact f32 values")]
    #![allow(
        clippy::type_complexity,
        reason = "the device tests carry literal case tables"
    )]

    use super::*;
    use crate::stats::NoiseModel;
    use crate::Trace;

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
            NoiseModel::bootstrap(vec![0.1, -0.2, 0.3, 0.0]).unwrap(),
            NoiseInteraction::Additive,
        );
        let err = pack_noise_params(&["x".to_string()], &lifting).unwrap_err();
        assert!(matches!(
            err,
            GpuMcError::UnsupportedNoiseFamily {
                family: "bootstrap"
            }
        ));
    }

    #[test]
    fn packs_the_inverse_transform_families() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "w",
            NoiseModel::weibull(2.0, 3.0).unwrap(),
            NoiseInteraction::Additive,
        );
        lifting.register(
            "c",
            NoiseModel::cauchy(1.0, 0.5).unwrap(),
            NoiseInteraction::Additive,
        );
        let packed = pack_noise_params(&["c".to_string(), "w".to_string()], &lifting).unwrap();
        assert_eq!(&packed[0..4], &[9.0f32, 0.0, 1.0, 0.5]); // Cauchy: location 1, scale 0.5
        assert_eq!(&packed[8..12], &[6.0f32, 0.0, 2.0, 3.0]); // Weibull: shape 2, scale 3
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
    fn a_temporal_formula_assembles_a_temporal_shader() {
        let formula = Formula::parse("always[0, 2](x > 0)").unwrap();
        let symbols = formula.variables();
        let (source, state_size) = build_temporal_shader(&formula, &symbols, 4).unwrap();
        assert_eq!(state_size, 1);
        assert!(source.contains("fn evaluate_temporal"));
        assert!(source.contains("@group(0) @binding(5) var<storage, read> times_buf"));
        assert!(source.contains("results[t] = evaluate_temporal(&trace, &times)"));
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn context_builds_on_a_device() {
        let formula = Formula::parse("x > 0").unwrap();
        let symbols = formula.variables();
        let (shader, _) = build_count_shader(&formula, &symbols).unwrap();
        assert!(GpuMcContext::new(&shader, false).is_ok());
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn temporal_count_runs_on_a_device() {
        let formula = Formula::parse("always[0, 2](x > 0)").unwrap();
        let symbols = formula.variables();
        let (shader, state_size) = build_temporal_shader(&formula, &symbols, 3).unwrap();
        assert_eq!(state_size, 1);
        let ctx = GpuMcContext::new(&shader, true).unwrap();
        let base_trace = [1.0f32, 1.0, 1.0];
        let times = [0.0f32, 1.0, 2.0];
        let noise = [0.0f32; NOISE_RECORD];
        let n = 100_000;
        let count = ctx
            .gpu_satisfaction_count(&base_trace, &noise, Some(&times), n, 7)
            .unwrap();
        assert_eq!(
            count, n,
            "a deterministic always holds on every realization"
        );
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    #[allow(
        clippy::cast_precision_loss,
        reason = "the count and sample size are far below 2^24"
    )]
    fn satisfaction_count_matches_a_known_probability() {
        let formula = Formula::parse("x > 0").unwrap();
        let symbols = formula.variables();
        let (shader, state_size) = build_count_shader(&formula, &symbols).unwrap();
        assert_eq!(state_size, 1);
        let ctx = GpuMcContext::new(&shader, false).unwrap();
        let base = [0.0f32];
        let mut noise = [0.0f32; NOISE_RECORD];
        noise[0] = 1.0; // Gaussian family
        noise[3] = 1.0; // standard deviation 1, mean stays 0
        let n = 1_000_000;
        let count = ctx
            .gpu_satisfaction_count(&base, &noise, None, n, 42)
            .unwrap();
        let p = count as f64 / n as f64;
        assert!((p - 0.5).abs() < 0.005, "expected about 0.5, got {p}");
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the GPU evaluates in f32; the test signals are small integers"
    )]
    fn temporal_verdict(
        formula_str: &str,
        times: &[f64],
        signals: &[(&str, &[f64])],
    ) -> (f64, u64, u64) {
        let formula = Formula::parse(formula_str).unwrap();
        let symbols = formula.variables();
        let mut trace = Trace::new(times.to_vec()).unwrap();
        for (name, values) in signals {
            trace.add_signal(name, values.to_vec()).unwrap();
        }
        let cpu = formula.robustness(&trace).unwrap();
        let (shader, _) = build_temporal_shader(&formula, &symbols, times.len()).unwrap();
        let ctx = GpuMcContext::new(&shader, true).unwrap();
        let mut base = Vec::new();
        for name in &symbols {
            let values = signals.iter().find(|(n, _)| *n == name.as_str()).unwrap().1;
            base.extend(values.iter().map(|&v| v as f32));
        }
        let grid: Vec<f32> = times.iter().map(|&t| t as f32).collect();
        let noise = vec![0.0f32; symbols.len() * NOISE_RECORD];
        let n = 100_000u64;
        let count = ctx
            .gpu_satisfaction_count(&base, &noise, Some(&grid), n, 1)
            .unwrap();
        (cpu, count, n)
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn temporal_robustness_sign_matches_the_cpu_monitor() {
        let cases: &[(&str, &[f64], &[(&str, &[f64])])] = &[
            (
                "always[0, 2](x > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 0.5, 3.0])],
            ),
            (
                "always[0, 2](x > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, -0.5, 2.0, 3.0])],
            ),
            (
                "eventually[0, 2](x > 5)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 6.0, 3.0])],
            ),
            (
                "eventually[0, 1](x > 5)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 6.0, 3.0])],
            ),
            (
                "(x > 0) until[0, 3] (y > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 1.0, 1.0, 1.0]), ("y", &[-1.0, -1.0, 2.0, 1.0])],
            ),
            (
                "(x > 0) until[0, 1] (y > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 1.0, 1.0, 1.0]), ("y", &[-1.0, -1.0, 2.0, 1.0])],
            ),
            (
                "historically[0, 2](x > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 3.0, 4.0])],
            ),
            (
                "once[0, 2](x > 5)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[1.0, 2.0, 3.0, 6.0])],
            ),
            (
                "next(x > 0)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[-1.0, 2.0, 3.0, 4.0])],
            ),
            (
                "always[0, 3](eventually[0, 1](x > 2))",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[3.0, 1.0, 3.0, 1.0])],
            ),
        ];
        for (formula, times, signals) in cases {
            let (cpu, count, n) = temporal_verdict(formula, times, signals);
            assert!(
                count == 0 || count == n,
                "zero-noise ensemble split on `{formula}`: {count}/{n}"
            );
            assert_eq!(
                count == n,
                cpu >= 0.0,
                "verdict mismatch on `{formula}`: cpu robustness {cpu}, gpu count {count}/{n}"
            );
        }
    }
}