//! GPU adaptive multilevel splitting for rare-event probabilities.

use core::fmt::Write as _;

use pollster::FutureExt as _;
use rand::{Rng as _, SeedableRng as _};
use rand_chacha::ChaCha8Rng;
use wgpu::util::DeviceExt as _;

use super::monte_carlo::{write_draw_residual, GpuMcError, NOISE_RECORD, PRNG_WGSL};
use super::transpiler::{emit_formula, validate, Ssa};
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::stats::{GpuSampler, NoiseModel, RareEventConfig, SimExpr, SimModel};

/// The fewest particles a splitting run accepts.
const MIN_PARTICLES: usize = 16;

/// The most splitting levels a run resolves before stopping.
const MAX_LEVELS: u32 = 64;

/// The fraction of the population kept at each level, the variance-optimal one half.
const KEEP_NUMERATOR: usize = 1;
const KEEP_DENOMINATOR: usize = 2;

/// Packs the model's noise sources into the device buffer, one record per source.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the device runs in f32; the family tag and parameters fit it"
)]
fn pack_sim_noise(noise: &[NoiseModel]) -> core::result::Result<Vec<f32>, GpuMcError> {
    let mut packed = vec![0.0f32; noise.len().max(1) * NOISE_RECORD];
    for (id, model) in noise.iter().enumerate() {
        let (family, params) = match model.gpu_sampler() {
            GpuSampler::Device { family, params } => (family, params),
            GpuSampler::Cpu { family } => {
                return Err(GpuMcError::UnsupportedNoiseFamily { family })
            }
        };
        let base = id * NOISE_RECORD;
        packed[base] = family as f32;
        for (i, &p) in params.iter().enumerate() {
            packed[base + 2 + i] = p as f32;
        }
    }
    Ok(packed)
}

/// Lowers a [`SimExpr`] to a WGSL value.
fn emit_sim_expr(ssa: &mut Ssa, expr: &SimExpr) -> Result<String> {
    match expr {
        SimExpr::Prev(d) => Ok(ssa.bind(&format!("(*prev)[{d}u]"))),
        SimExpr::Time => Ok(ssa.bind("t")),
        SimExpr::Const(c) => Ok(ssa.bind(&format!("f32({c:?})"))),
        SimExpr::Add(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("{l} + {r}")))
        }
        SimExpr::Sub(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("{l} - {r}")))
        }
        SimExpr::Mul(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("{l} * {r}")))
        }
        SimExpr::Div(a, b) => {
            let (l, r) = (emit_sim_expr(ssa, a)?, emit_sim_expr(ssa, b)?);
            Ok(ssa.bind(&format!("select({l} / {r}, 1e38, abs({r}) < 1e-9)")))
        }
        SimExpr::Call(name, args) => emit_sim_call(ssa, name, args),
        SimExpr::Noise(id) => Ok(ssa.bind(&format!("draw_residual({id}u, rng)"))),
    }
}

/// Lowers a whitelisted function call.
fn emit_sim_call(ssa: &mut Ssa, name: &str, args: &[SimExpr]) -> Result<String> {
    let lowered = args
        .iter()
        .map(|arg| emit_sim_expr(ssa, arg))
        .collect::<Result<Vec<_>>>()?;
    let value = match (name, lowered.as_slice()) {
        ("abs", [a]) => format!("abs({a})"),
        ("sqrt", [a]) => format!("sqrt(max({a}, 0.0))"),
        ("exp", [a]) => format!("exp(clamp({a}, -87.0, 87.0))"),
        ("ln", [a]) => format!("log(max({a}, 1e-38))"),
        ("log", [a]) => format!("log(max({a}, 1e-38)) / 2.302585093"),
        ("sin", [a]) => format!("sin({a})"),
        ("cos", [a]) => format!("cos({a})"),
        ("tan", [a]) => format!("tan({a})"),
        ("floor", [a]) => format!("floor({a})"),
        ("ceil", [a]) => format!("ceil({a})"),
        ("min", [a, b]) => format!("min({a}, {b})"),
        ("max", [a, b]) => format!("max({a}, {b})"),
        ("pow", [a, b]) => format!("pow({a}, {b})"),
        _ => {
            return Err(Error::Transpilation {
                message: format!(
                    "the splitting path cannot lower `{name}` with {} arguments",
                    args.len()
                ),
            })
        }
    };
    Ok(ssa.bind(&value))
}

/// Emits `advance`, `init_state`, and `psi_margin`.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when an expression or `psi` cannot be lowered.
fn build_dynamics(model: &SimModel, psi: &Formula, symbols: &[String]) -> Result<String> {
    let v = symbols.len().max(1);
    let mut source = String::from(PRNG_WGSL);
    write_draw_residual(&mut source);

    let mut advance_ssa = Ssa::new();
    let mut advance_assigns = String::new();
    for (d, expr) in model.advance_exprs().iter().enumerate() {
        let result = emit_sim_expr(&mut advance_ssa, expr)?;
        let _ = writeln!(advance_assigns, "    out[{d}u] = {result};");
    }
    let _ = writeln!(
        source,
        "\nfn advance(prev: ptr<function, array<f32, {v}>>, t: f32, rng: ptr<function, u32>) -> array<f32, {v}> {{\n    var out: array<f32, {v}>;\n{body}{advance_assigns}    return out;\n}}",
        body = advance_ssa.body,
    );

    let mut init_ssa = Ssa::new();
    let mut init_assigns = String::new();
    for (d, expr) in model.init_exprs().iter().enumerate() {
        let result = emit_sim_expr(&mut init_ssa, expr)?;
        let _ = writeln!(init_assigns, "    out[{d}u] = {result};");
    }
    let _ = writeln!(
        source,
        "\nfn init_state(rng: ptr<function, u32>) -> array<f32, {v}> {{\n    let t = 0.0;\n    var out: array<f32, {v}>;\n{body}{init_assigns}    return out;\n}}",
        body = init_ssa.body,
    );

    let mut margin_ssa = Ssa::new();
    let margin = emit_formula(&mut margin_ssa, psi, symbols, &|slot| {
        format!("(*state)[{slot}u]")
    })?;
    let _ = writeln!(
        source,
        "\nfn psi_margin(state: ptr<function, array<f32, {v}>>) -> f32 {{\n{body}    return {margin};\n}}",
        body = margin_ssa.body,
    );
    Ok(source)
}

/// Emits the roll kernel: one thread per particle rolls its trajectory and tracks `z`, the running maximum violation.
fn write_roll_kernel(source: &mut String, v: usize, dt: f64) {
    let _ = write!(
        source,
        "\n@compute @workgroup_size(256)\nfn roll_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {{\n    let p = gid.x;\n    if (p >= params.n_particles) {{ return; }}\n    let wl = params.window_len;\n    var rng = init_rng(p + params.level_round * params.n_particles, params.seed);\n    var state = init_state(&rng);\n    var z = bitcast<f32>(0xff800000u);\n    let base = p * wl * {v}u;\n    for (var k = 0u; k < wl; k = k + 1u) {{\n        for (var d = 0u; d < {v}u; d = d + 1u) {{ trajectory[base + k * {v}u + d] = state[d]; }}\n        z = max(z, -psi_margin(&state));\n        if (k + 1u < wl) {{\n            let t = f32(k) * f32({dt:?});\n            state = advance(&state, t, &rng);\n        }}\n    }}\n    z_buf[p] = z;\n}}\n"
    );
}

/// Emits the resample kernel: a non-survivor clones the survivor named in `assignment` from where it first crosses the level.
fn write_resample_kernel(source: &mut String, v: usize, dt: f64) {
    let _ = write!(
        source,
        "\n@compute @workgroup_size(256)\nfn resample_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {{\n    let p = gid.x;\n    if (p >= params.n_particles) {{ return; }}\n    let s = assignment[p];\n    if (s == p) {{ return; }}\n    let wl = params.window_len;\n    let s_base = s * wl * {v}u;\n    let p_base = p * wl * {v}u;\n    var running = bitcast<f32>(0xff800000u);\n    var crossing = wl - 1u;\n    for (var k = 0u; k < wl; k = k + 1u) {{\n        var st: array<f32, {v}>;\n        for (var d = 0u; d < {v}u; d = d + 1u) {{ st[d] = trajectory[s_base + k * {v}u + d]; }}\n        running = max(running, -psi_margin(&st));\n        if (running >= params.level) {{ crossing = k; break; }}\n    }}\n    for (var k = 0u; k <= crossing; k = k + 1u) {{\n        for (var d = 0u; d < {v}u; d = d + 1u) {{ trajectory[p_base + k * {v}u + d] = trajectory[s_base + k * {v}u + d]; }}\n    }}\n    var state: array<f32, {v}>;\n    for (var d = 0u; d < {v}u; d = d + 1u) {{ state[d] = trajectory[s_base + crossing * {v}u + d]; }}\n    var rng = init_rng(p + params.level_round * params.n_particles, params.seed);\n    var z = running;\n    for (var k = crossing + 1u; k < wl; k = k + 1u) {{\n        let t = f32(k - 1u) * f32({dt:?});\n        state = advance(&state, t, &rng);\n        for (var d = 0u; d < {v}u; d = d + 1u) {{ trajectory[p_base + k * {v}u + d] = state[d]; }}\n        z = max(z, -psi_margin(&state));\n    }}\n    z_buf[p] = z;\n}}\n"
    );
}

/// Assembles the full splitting shader.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when the dynamics or `psi` cannot be lowered, or the shader does not validate.
fn build_splitting_shader(
    model: &SimModel,
    psi: &Formula,
    symbols: &[String],
) -> Result<(String, usize)> {
    let mut source = String::from(
        "struct Params {\n    n_particles: u32,\n    window_len: u32,\n    seed: u32,\n    level_round: u32,\n    level: f32,\n}\n\n@group(0) @binding(0) var<uniform> params: Params;\n@group(0) @binding(1) var<storage, read_write> trajectory: array<f32>;\n@group(0) @binding(2) var<storage, read_write> z_buf: array<f32>;\n@group(0) @binding(3) var<storage, read> noise_params: array<f32>;\n@group(0) @binding(4) var<storage, read> assignment: array<u32>;\n\n",
    );
    source.push_str(&build_dynamics(model, psi, symbols)?);
    let v = symbols.len().max(1);
    let dt = model.dt();
    write_roll_kernel(&mut source, v, dt);
    write_resample_kernel(&mut source, v, dt);
    validate(&source)?;
    Ok((source, v))
}

/// The outcome of a GPU rare-event splitting run.
///
/// `probability` is the fixed-effort multilevel-splitting estimate. It is
/// consistent as `particles` grows but carries an `O(levels / particles)` bias,
/// unlike the unbiased CPU last-particle estimate in
/// [`RareEventResult`](crate::stats::RareEventResult); it is a distinct type so the
/// two are never read as the same number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSplittingEstimate {
    /// The estimated probability of the rare violation.
    pub probability: f64,
    /// The particle population the run used.
    pub particles: usize,
    /// The number of splitting levels the run resolved.
    pub levels: u32,
}

/// The uniform block the kernels read, padded to a 16-byte uniform alignment.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SplitParams {
    n_particles: u32,
    window_len: u32,
    seed: u32,
    level_round: u32,
    level: f32,
    pad: [u32; 3],
}

/// Builds the per-particle survivor assignment for a resample.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the particle count stays below 2^32"
)]
fn build_assignment(z: &[f32], level: f32, seed: u32, round: u32) -> Vec<u32> {
    let survivors: Vec<u32> = (0..z.len())
        .filter(|&i| z[i] >= level)
        .map(|i| i as u32)
        .collect();
    let mut rng = ChaCha8Rng::seed_from_u64(u64::from(seed) ^ (u64::from(round) << 40));
    (0..z.len())
        .map(|i| {
            if z[i] >= level {
                i as u32
            } else {
                survivors[rng.random_range(0..survivors.len())]
            }
        })
        .collect()
}

/// A device and the two pipelines that roll and resample a particle population.
struct GpuSplittingContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    roll: wgpu::ComputePipeline,
    resample: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl GpuSplittingContext {
    /// Builds the context from the assembled splitting shader.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Gpu`] when no device is available or the shader does not compile.
    fn new(shader_source: &str) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
            .map_err(|_| Error::Gpu {
                message: "no compatible GPU adapter for the splitting path".into(),
            })?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sentil splitting"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::default(),
            })
            .block_on()
            .map_err(|e| Error::Gpu {
                message: format!("could not create a GPU device: {e}"),
            })?;

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sentil splitting shader"),
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
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sentil splitting layout"),
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
            label: Some("sentil splitting pipeline layout"),
            bind_group_layouts: &[&layout],
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
        let roll = pipeline("sentil roll", "roll_kernel");
        let resample = pipeline("sentil resample", "resample_kernel");
        if let Some(err) = device.pop_error_scope().block_on() {
            return Err(Error::Gpu {
                message: format!("the splitting shader did not compile: {err}"),
            });
        }
        Ok(Self {
            device,
            queue,
            roll,
            resample,
            layout,
        })
    }

    /// Runs fixed-effort multilevel splitting over the population.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Gpu`] when a dispatch or readback fails.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "the particle count and window length stay far below 2^24"
    )]
    fn run_splitting(
        &self,
        noise: &[f32],
        n: usize,
        window_len: usize,
        v: usize,
        margin: f32,
        seed: u32,
    ) -> Result<GpuSplittingEstimate> {
        let n_bytes = n as u64 * 4;
        let storage = |label: &str, size: u64, extra: wgpu::BufferUsages| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | extra,
                mapped_at_creation: false,
            })
        };
        let trajectory = storage(
            "trajectory",
            (n * window_len * v) as u64 * 4,
            wgpu::BufferUsages::empty(),
        );
        let z = storage("z", n_bytes, wgpu::BufferUsages::COPY_SRC);
        let assignment = storage("assignment", n_bytes, wgpu::BufferUsages::COPY_DST);
        let noise_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("noise"),
                contents: bytemuck::cast_slice(noise),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let params_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("split params"),
            size: core::mem::size_of::<SplitParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let z_readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("z readback"),
            size: n_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("split bind group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: trajectory.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: z.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: noise_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: assignment.as_entire_binding(),
                },
            ],
        });

        self.write_params(&params_buf, n, window_len, seed, 0, f32::NEG_INFINITY);
        let mut z_host = self.dispatch_and_read(&bind_group, &self.roll, n, &z, &z_readback)?;

        let keep = (n * KEEP_NUMERATOR / KEEP_DENOMINATOR).max(1);
        let mut probability = 1.0_f64;
        let mut round = 1_u32;
        loop {
            let mut sorted = z_host.clone();
            sorted.sort_unstable_by(|a, b| b.total_cmp(a));
            let level = sorted[keep - 1];
            let survivors = z_host.iter().filter(|&&zi| zi >= level).count();
            if level >= margin || survivors == n || round > MAX_LEVELS {
                let reached = z_host.iter().filter(|&&zi| zi >= margin).count();
                probability *= reached as f64 / n as f64;
                return Ok(GpuSplittingEstimate {
                    probability,
                    particles: n,
                    levels: round - 1,
                });
            }
            probability *= survivors as f64 / n as f64;
            let assignment_host = build_assignment(&z_host, level, seed, round);
            self.queue
                .write_buffer(&assignment, 0, bytemuck::cast_slice(&assignment_host));
            self.write_params(&params_buf, n, window_len, seed, round, level);
            z_host = self.dispatch_and_read(&bind_group, &self.resample, n, &z, &z_readback)?;
            round += 1;
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the particle count and window length stay far below 2^32"
    )]
    fn write_params(
        &self,
        buf: &wgpu::Buffer,
        n: usize,
        window_len: usize,
        seed: u32,
        level_round: u32,
        level: f32,
    ) {
        let params = SplitParams {
            n_particles: n as u32,
            window_len: window_len as u32,
            seed,
            level_round,
            level,
            pad: [0; 3],
        };
        self.queue.write_buffer(buf, 0, bytemuck::bytes_of(&params));
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the particle count stays far below 2^32"
    )]
    fn dispatch_and_read(
        &self,
        bind_group: &wgpu::BindGroup,
        pipeline: &wgpu::ComputePipeline,
        n: usize,
        z: &wgpu::Buffer,
        z_readback: &wgpu::Buffer,
    ) -> Result<Vec<f32>> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("split encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("split pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups((n as u32).div_ceil(256), 1, 1);
        }
        encoder.copy_buffer_to_buffer(z, 0, z_readback, 0, n as u64 * 4);
        let submission = self.queue.submit(Some(encoder.finish()));

        let slice = z_readback.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|e| Error::Gpu {
                message: format!("GPU poll failed: {e}"),
            })?;
        match rx.block_on() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(Error::Gpu {
                    message: format!("readback map failed: {e}"),
                })
            }
            Err(e) => {
                return Err(Error::Gpu {
                    message: format!("readback channel failed: {e}"),
                })
            }
        }
        let data = slice.get_mapped_range();
        let scores: Vec<f32> = data
            .chunks_exact(4)
            .take(n)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        drop(data);
        z_readback.unmap();
        Ok(scores)
    }
}

/// Estimates the rare violation probability of `psi` inside an `always` window over the `model` dynamics.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when the model or `psi` cannot be lowered, and [`Error::Gpu`] when no device is present or a dispatch fails.
fn gpu_split(
    model: &SimModel,
    psi: &Formula,
    symbols: &[String],
    n: usize,
    window_len: usize,
    margin: f32,
    seed: u32,
) -> Result<GpuSplittingEstimate> {
    let noise = pack_sim_noise(model.noise()).map_err(Error::from)?;
    let (shader, v) = build_splitting_shader(model, psi, symbols)?;
    let context = GpuSplittingContext::new(&shader)?;
    context.run_splitting(&noise, n, window_len, v, margin, seed)
}

/// The window length over the model's grid, capped at the horizon.
fn window_length(dt: f64, horizon: usize, upper: Option<f64>) -> usize {
    let Some(b) = upper else {
        return horizon + 1;
    };
    let mut t = 0.0;
    let mut len = 1;
    for _ in 0..horizon {
        t += dt;
        if t <= b {
            len += 1;
        } else {
            break;
        }
    }
    len
}

impl Formula {
    /// Estimates `P~p(phi)` over a GPU-transpilable `model` by multilevel splitting,
    /// for a violation too rare for plain Monte Carlo to resolve.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotProbabilistic`] unless the formula is `P~p(phi)`, [`Error::Transpilation`] when the inner formula is not an `always[0, b]` over an atemporal predicate, [`Error::InvalidConfig`] for too few particles, and [`Error::Gpu`] when no device is present.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the margin narrows to f32 by contract and the seed narrowing is intentional"
    )]
    pub fn check_rare_event_gpu(
        &self,
        model: &SimModel,
        config: &RareEventConfig,
    ) -> Result<GpuSplittingEstimate> {
        let Formula::Probabilistic(_, _, inner) = self else {
            return Err(Error::NotProbabilistic);
        };
        let Formula::Always(interval, psi) = inner.as_ref() else {
            return Err(Error::Transpilation {
                message: "the GPU splitter needs an always-shaped inner formula; run \
                          check_rare_event on the CPU"
                    .into(),
            });
        };
        if interval.lower > 0.0 {
            return Err(Error::Transpilation {
                message: format!(
                    "the GPU splitter needs an always window starting at 0, got lower bound {}; \
                     run check_rare_event on the CPU",
                    interval.lower
                ),
            });
        }
        if psi.has_temporal() {
            return Err(Error::Transpilation {
                message: "the GPU splitter needs an atemporal inner predicate; run \
                          check_rare_event on the CPU"
                    .into(),
            });
        }
        if config.particles < MIN_PARTICLES {
            return Err(Error::InvalidConfig {
                context: "gpu splitting",
                message: format!(
                    "at least {MIN_PARTICLES} particles are required, got {}",
                    config.particles
                ),
            });
        }
        let symbols = psi.variables();
        let window = window_length(model.dt(), model.horizon(), interval.upper);
        gpu_split(
            model,
            psi,
            &symbols,
            config.particles,
            window,
            config.margin as f32,
            config.seed as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the packed records are exact f32 values")]

    use super::*;
    use crate::stats::NoiseModel;

    fn boxed(expr: SimExpr) -> Box<SimExpr> {
        Box::new(expr)
    }

    fn validated_dynamics(model: &SimModel, psi: &Formula, symbols: &[String]) -> String {
        let dynamics = build_dynamics(model, psi, symbols).unwrap();
        let source = format!(
            "@group(0) @binding(4) var<storage, read> noise_params: array<f32>;\n\n{dynamics}"
        );
        validate(&source).unwrap();
        source
    }

    fn walk_model() -> (SimModel, Formula, Vec<String>) {
        let advance = SimExpr::Add(
            boxed(SimExpr::Add(
                boxed(SimExpr::Prev(0)),
                boxed(SimExpr::Const(0.05)),
            )),
            boxed(SimExpr::Noise(0)),
        );
        let model = SimModel::new(
            ["x"],
            1.0,
            32,
            vec![SimExpr::Const(0.0)],
            vec![advance],
            vec![NoiseModel::gaussian(0.0, 1.0).unwrap()],
        )
        .unwrap();
        let psi = Formula::parse("x > -8").unwrap();
        (model, psi, vec!["x".to_owned()])
    }

    #[test]
    fn the_splitting_shader_assembles_and_validates() {
        let (model, psi, symbols) = walk_model();
        let (source, v) = build_splitting_shader(&model, &psi, &symbols).unwrap();
        assert_eq!(v, 1);
        assert!(source.contains("fn roll_kernel"));
        assert!(source.contains("fn resample_kernel"));
        assert!(source.contains("z = max(z, -psi_margin(&state))"));
        assert!(source.contains("if (s == p) { return; }"));
        assert!(source.contains("if (running >= params.level) { crossing = k; break; }"));
    }

    #[test]
    fn the_assignment_keeps_survivors_and_clones_from_survivors() {
        let z = [5.0_f32, 1.0, 6.0, 0.5, 7.0];
        let assignment = build_assignment(&z, 5.0, 42, 1);
        assert_eq!(assignment, [0, assignment[1], 2, assignment[3], 4]);
        let survivors = [0_u32, 2, 4];
        assert!(survivors.contains(&assignment[1]));
        assert!(survivors.contains(&assignment[3]));
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn the_splitter_runs_on_a_device_and_returns_a_probability() {
        let (model, psi, symbols) = walk_model();
        let estimate = gpu_split(&model, &psi, &symbols, 4096, 33, 0.0, 7).unwrap();
        assert!(
            (0.0..=1.0).contains(&estimate.probability),
            "probability {}",
            estimate.probability
        );
        assert_eq!(estimate.particles, 4096);
        assert!(estimate.levels > 0, "expected several splitting levels");
    }

    #[test]
    fn window_length_accumulates_the_grid() {
        assert_eq!(window_length(1.0, 32, None), 33);
        assert_eq!(window_length(1.0, 32, Some(5.0)), 6);
        assert_eq!(window_length(0.5, 10, Some(2.0)), 5);
        assert_eq!(window_length(1.0, 4, Some(100.0)), 5);
    }

    #[test]
    fn the_entry_declines_unsupported_runs() {
        let (model, ..) = walk_model();
        let config = RareEventConfig::default();
        let cases = [
            "always[0, 5](x > 0)",
            "P>=0.5(eventually[0, 5](x > 0))",
            "P>=0.5(always[2, 5](x > 0))",
            "P>=0.5(always[0, 5](eventually[0, 1](x > 0)))",
        ];
        let not_p = Formula::parse(cases[0]).unwrap();
        assert!(matches!(
            not_p.check_rare_event_gpu(&model, &config),
            Err(Error::NotProbabilistic)
        ));
        for case in &cases[1..] {
            let phi = Formula::parse(case).unwrap();
            assert!(
                matches!(
                    phi.check_rare_event_gpu(&model, &config),
                    Err(Error::Transpilation { .. })
                ),
                "expected `{case}` to decline to the CPU"
            );
        }
        let supported = Formula::parse("P>=0.5(always[0, 5](x > -8))").unwrap();
        let few = RareEventConfig {
            particles: 4,
            ..RareEventConfig::default()
        };
        assert!(matches!(
            supported.check_rare_event_gpu(&model, &few),
            Err(Error::InvalidConfig { .. })
        ));
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn the_public_entry_runs_on_a_device() {
        let (model, ..) = walk_model();
        let phi = Formula::parse("P>=0.5(always[0, 100](x > -8))").unwrap();
        let config = RareEventConfig {
            particles: 4096,
            margin: 0.0,
            seed: 11,
        };
        let estimate = phi.check_rare_event_gpu(&model, &config).unwrap();
        assert!((0.0..=1.0).contains(&estimate.probability));
    }

    #[test]
    fn a_noisy_advance_lowers_and_validates() {
        let advance = SimExpr::Add(
            boxed(SimExpr::Add(
                boxed(SimExpr::Prev(0)),
                boxed(SimExpr::Const(0.1)),
            )),
            boxed(SimExpr::Noise(0)),
        );
        let model = SimModel::new(
            ["x"],
            1.0,
            8,
            vec![SimExpr::Const(0.0)],
            vec![advance],
            vec![NoiseModel::gaussian(0.0, 1.0).unwrap()],
        )
        .unwrap();
        let psi = Formula::parse("x > 0").unwrap();
        let source = validated_dynamics(&model, &psi, &["x".to_owned()]);
        assert!(source.contains("fn advance(prev: ptr<function, array<f32, 1>>"));
        assert!(source.contains("draw_residual(0u, rng)"));
        assert!(source.contains("(*prev)[0u]"));
        assert!(source.contains("fn psi_margin"));
    }

    #[test]
    fn calls_and_two_variables_lower_and_validate() {
        let advance_x = SimExpr::Add(
            boxed(SimExpr::Prev(0)),
            boxed(SimExpr::Mul(
                boxed(SimExpr::Const(0.5)),
                boxed(SimExpr::Prev(1)),
            )),
        );
        let advance_y = SimExpr::Add(
            boxed(SimExpr::Call("cos".to_owned(), vec![SimExpr::Time])),
            boxed(SimExpr::Noise(0)),
        );
        let model = SimModel::new(
            ["x", "y"],
            0.5,
            6,
            vec![SimExpr::Const(0.0), SimExpr::Const(1.0)],
            vec![advance_x, advance_y],
            vec![NoiseModel::gaussian(0.0, 0.2).unwrap()],
        )
        .unwrap();
        let psi = Formula::parse("x > 0 and y < 5").unwrap();
        let source = validated_dynamics(&model, &psi, &["x".to_owned(), "y".to_owned()]);
        assert!(source.contains("array<f32, 2>"));
        assert!(source.contains("cos("));
    }

    #[test]
    fn an_init_expression_using_time_lowers_with_time_zero() {
        let model = SimModel::new(
            ["x"],
            1.0,
            4,
            vec![SimExpr::Call("cos".to_owned(), vec![SimExpr::Time])],
            vec![SimExpr::Prev(0)],
            vec![],
        )
        .unwrap();
        let psi = Formula::parse("x > 0").unwrap();
        let source = validated_dynamics(&model, &psi, &["x".to_owned()]);
        assert!(source.contains("let t = 0.0;"));
    }

    #[test]
    fn a_temporal_psi_cannot_be_lowered() {
        let model = SimModel::new(
            ["x"],
            1.0,
            4,
            vec![SimExpr::Const(0.0)],
            vec![SimExpr::Prev(0)],
            vec![],
        )
        .unwrap();
        let psi = Formula::parse("always[0, 2](x > 0)").unwrap();
        let result = build_dynamics(&model, &psi, &["x".to_owned()]);
        assert!(matches!(result, Err(Error::Transpilation { .. })));
    }

    #[test]
    fn the_noise_buffer_packs_one_record_per_source() {
        let packed = pack_sim_noise(&[
            NoiseModel::gaussian(0.5, 2.0).unwrap(),
            NoiseModel::exponential(1.5).unwrap(),
        ])
        .unwrap();
        assert_eq!(packed.len(), 2 * NOISE_RECORD);
        // Gaussian is family 1 with mean and standard deviation in slots 2 and 3.
        assert_eq!(packed[0], 1.0);
        assert_eq!(packed[2], 0.5);
        assert_eq!(packed[3], 2.0);
        assert_eq!(packed[NOISE_RECORD + 2], 1.5);
    }
}