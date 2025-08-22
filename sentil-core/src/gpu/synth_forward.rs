//! GPU batch scoring of the smooth robustness for synthesis.

use core::fmt::Write as _;

use pollster::FutureExt as _;
use wgpu::util::DeviceExt as _;

use super::temporal::transpile_temporal_soft;
use super::transpiler::validate;
use crate::error::{Error, Result};
use crate::formula::Formula;

/// Threads per workgroup, one per candidate.
const WORKGROUP_SIZE: u32 = 256;

/// The most candidates one dispatch scores, bounded by the 65535-workgroup limit.
const MAX_BATCH: usize = 65535 * WORKGROUP_SIZE as usize;

/// The uniform block, padded to a 16-byte uniform alignment.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ForwardParams {
    n_candidates: u32,
    pad: [u32; 3],
}

/// Assembles the batch-scoring shader for a soft `formula`.
///
/// # Errors
///
/// Returns [`Error::Transpilation`] when the formula cannot be lowered or the shader does not validate.
pub(crate) fn build_soft_forward_shader(
    formula: &Formula,
    symbols: &[String],
    trace_len: usize,
    beta: f64,
) -> Result<(String, usize)> {
    let shader = transpile_temporal_soft(formula, symbols, trace_len, beta)?;
    let v = shader.state_size.max(1);
    let l = trace_len;
    let stride = v * l;
    let mut source = String::from(
        "struct Params {\n    n_candidates: u32,\n}\n\n@group(0) @binding(0) var<uniform> params: Params;\n@group(0) @binding(1) var<storage, read> batch: array<f32>;\n@group(0) @binding(2) var<storage, read> times_buf: array<f32>;\n@group(0) @binding(3) var<storage, read_write> results: array<f32>;\n\n",
    );
    source.push_str(&shader.evaluate_temporal);
    let _ = write!(
        source,
        "\n\n@compute @workgroup_size({WORKGROUP_SIZE})\nfn score_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {{\n    let b = gid.x;\n    if (b >= params.n_candidates) {{\n        return;\n    }}\n    var trace: array<array<f32, {l}>, {v}>;\n    var times: array<f32, {l}>;\n    for (var i = 0u; i < {l}u; i = i + 1u) {{\n        times[i] = times_buf[i];\n        for (var s = 0u; s < {v}u; s = s + 1u) {{\n            trace[s][i] = batch[b * {stride}u + s * {l}u + i];\n        }}\n    }}\n    results[b] = evaluate_temporal(&trace, &times);\n}}\n"
    );
    validate(&source)?;
    Ok((source, shader.state_size))
}

/// A GPU context that scores a batch of candidate trajectories with the soft robustness.
pub(crate) struct SynthForwardContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl SynthForwardContext {
    /// Builds the context from the assembled batch-scoring shader.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Gpu`] when no adapter or device is available, or the shader does not compile.
    pub(crate) fn new(shader_source: &str) -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
            .map_err(|_| Error::Gpu {
                message: "no compatible GPU adapter for the synthesis batch path".into(),
            })?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sentil synthesis forward"),
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
            label: Some("sentil soft forward shader"),
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
            label: Some("sentil soft forward layout"),
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
                storage(1, true),
                storage(2, true),
                storage(3, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sentil soft forward pipeline layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sentil soft forward"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("score_kernel"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        if let Some(err) = device.pop_error_scope().block_on() {
            return Err(Error::Gpu {
                message: format!("the synthesis batch shader did not compile: {err}"),
            });
        }
        Ok(Self {
            device,
            queue,
            pipeline,
            layout,
        })
    }

    /// Scores `n_candidates` trajectories from a batch-major `batch` over the shared `times` grid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Gpu`] when the batch exceeds the dispatch limit or the readback fails.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "n_candidates is checked below MAX_BATCH, so it fits u32"
    )]
    pub(crate) fn score_batch(
        &self,
        batch: &[f32],
        times: &[f32],
        n_candidates: usize,
    ) -> Result<Vec<f32>> {
        if n_candidates == 0 {
            return Ok(Vec::new());
        }
        if n_candidates > MAX_BATCH {
            return Err(Error::Gpu {
                message: format!(
                    "{n_candidates} candidates exceeds the GPU batch limit of {MAX_BATCH}"
                ),
            });
        }
        let params = ForwardParams {
            n_candidates: n_candidates as u32,
            pad: [0; 3],
        };
        let init = |label: &str, contents: &[u8]| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("forward params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let batch_buffer = init("candidate batch", bytemuck::cast_slice(batch));
        let times_buffer = init("times", bytemuck::cast_slice(times));
        let result_bytes = n_candidates as u64 * 4;
        let results_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forward results"),
            size: result_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("forward readback"),
            size: result_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("forward bind group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: batch_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: times_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: results_buffer.as_entire_binding(),
                },
            ],
        });
        let num_workgroups = (n_candidates as u32).div_ceil(WORKGROUP_SIZE);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("forward encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("score"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(num_workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&results_buffer, 0, &readback_buffer, 0, result_bytes);
        let submission = self.queue.submit(Some(encoder.finish()));
        self.read_values(&readback_buffer, submission, n_candidates)
    }

    fn read_values(
        &self,
        readback: &wgpu::Buffer,
        submission: wgpu::SubmissionIndex,
        n: usize,
    ) -> Result<Vec<f32>> {
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
        let mut values = Vec::with_capacity(n);
        for chunk in data.chunks_exact(4).take(n) {
            let bytes: [u8; 4] = chunk.try_into().map_err(|_| Error::Gpu {
                message: "a robustness value was not four bytes".into(),
            })?;
            values.push(f32::from_le_bytes(bytes));
        }
        drop(data);
        readback.unmap();
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_forward_shader_assembles_and_validates() {
        let f = Formula::parse("always[0, 2](x > 0)").unwrap();
        let symbols = f.variables();
        let (src, state) = build_soft_forward_shader(&f, &symbols, 4, 10.0).unwrap();
        assert_eq!(state, 1);
        assert!(src.contains("fn score_kernel"));
        assert!(src.contains("@binding(1) var<storage, read> batch"));
        assert!(src.contains("results[b] = evaluate_temporal(&trace, &times)"));
        assert!(src.contains("fn soft_min2"));
    }

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn score_batch_runs_on_a_device() {
        let f = Formula::parse("always[0, 2](x > 0)").unwrap();
        let symbols = f.variables();
        let (src, _) = build_soft_forward_shader(&f, &symbols, 3, 10.0).unwrap();
        let ctx = SynthForwardContext::new(&src).unwrap();
        let batch = [1.0f32, 1.0, 1.0, 1.0, -2.0, 1.0];
        let times = [0.0f32, 1.0, 2.0];
        let out = ctx.score_batch(&batch, &times, 2).unwrap();
        assert_eq!(out.len(), 2);
        assert!(
            out[0] > 0.0,
            "all-positive candidate is satisfied: {}",
            out[0]
        );
        assert!(out[1] < 0.0, "dipping candidate is violated: {}", out[1]);
    }
}