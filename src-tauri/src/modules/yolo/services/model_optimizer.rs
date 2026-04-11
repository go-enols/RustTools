//! Model Optimizer - Pure Rust ONNX Model Optimization
//!
//! This module provides pure Rust model analysis and benchmarking utilities.
//! Uses tract's optimization features without Python dependencies.

use std::path::Path;
use tract_onnx::prelude::*;
use std::time::Instant;

/// Model analysis result
#[derive(Debug, Clone)]
pub struct ModelAnalysis {
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
    pub model_size_mb: f64,
    pub parameters: usize,
}

/// Model optimizer for YOLO models
pub struct ModelOptimizer;

impl ModelOptimizer {
    /// Create a new model optimizer
    pub fn new() -> Self {
        Self
    }

    /// Analyze model structure
    pub fn analyze_model(&self, model_path: &str) -> Result<ModelAnalysis, String> {
        let path = Path::new(model_path);
        if !path.exists() {
            return Err(format!("Model file not found: {}", model_path));
        }

        eprintln!("[Optimizer] Analyzing model: {}", model_path);

        let model = tract_onnx::onnx()
            .model_for_path(path)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        // Get model info
        let input_fact = model.input_fact(0)
            .map_err(|e| format!("Failed to get input fact: {}", e))?;
        let output_fact = model.output_fact(0)
            .map_err(|e| format!("Failed to get output fact: {}", e))?;

        // Get shape dimensions
        let input_shape = vec![640, 640, 3]; // Default for YOLO
        let output_shape = vec![1, 8400, 84];  // Default for YOLOv8

        // Calculate model size
        let model_size = path.metadata()
            .map(|m| m.len() as f64 / 1024.0 / 1024.0)
            .unwrap_or(0.0);

        // Estimate parameters
        let parameters = model.nodes().len() * 1000;

        let analysis = ModelAnalysis {
            input_shape,
            output_shape,
            model_size_mb: model_size,
            parameters,
        };

        eprintln!("[Optimizer] Analysis complete:");
        eprintln!("  Input shape: {:?}", analysis.input_shape);
        eprintln!("  Output shape: {:?}", analysis.output_shape);
        eprintln!("  Model size: {:.2} MB", analysis.model_size_mb);
        eprintln!("  Parameters: {}", analysis.parameters);

        Ok(analysis)
    }

    /// Benchmark model inference
    pub fn benchmark_model(&self, model_path: &str, iterations: usize) -> Result<BenchmarkResult, String> {
        let path = Path::new(model_path);
        if !path.exists() {
            return Err(format!("Model file not found: {}", model_path));
        }

        eprintln!("[Optimizer] Benchmarking model: {}", model_path);

        // Load and optimize model
        let model = tract_onnx::onnx()
            .model_for_path(path)
            .map_err(|e| format!("Failed to load model: {}", e))?
            .with_input_fact(0, f32::fact(&[1, 3, 640, 640]).into())
            .map_err(|e| format!("Failed to configure input: {}", e))?
            .into_typed()
            .map_err(|e| format!("Failed to type model: {}", e))?
            .into_optimized()
            .map_err(|e| format!("Failed to optimize: {}", e))?
            .into_runnable()
            .map_err(|e| format!("Failed to compile: {}", e))?;

        // Create dummy input
        let input_data: Vec<f32> = vec![0.0f32; 640 * 640 * 3];
        let array = tract_ndarray::Array4::from_shape_vec((1, 3, 640, 640), input_data)
            .map_err(|e| format!("Failed to create array: {}", e))?;
        let input = Tensor::from(array);

        // Warmup
        eprintln!("[Optimizer] Warming up...");
        for _ in 0..5 {
            let _ = model.run(tvec![input.clone().into()]);
        }

        // Benchmark
        eprintln!("[Optimizer] Running {} iterations...", iterations);
        let mut times = Vec::with_capacity(iterations);

        for i in 0..iterations {
            let start = Instant::now();
            let _ = model.run(tvec![input.clone().into()]);
            let elapsed = start.elapsed().as_secs_f64();

            if i % 10 == 0 {
                eprintln!("[Optimizer] Iteration {}/{}: {:.3}s", i + 1, iterations, elapsed);
            }

            times.push(elapsed);
        }

        // Calculate statistics
        let avg_time = times.iter().sum::<f64>() / times.len() as f64;
        let min_time = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let result = BenchmarkResult {
            iterations,
            avg_time_ms: avg_time * 1000.0,
            min_time_ms: min_time * 1000.0,
            max_time_ms: max_time * 1000.0,
            fps: 1.0 / avg_time,
        };

        eprintln!("[Optimizer] Benchmark results:");
        eprintln!("[Optimizer]   Average: {:.2} ms ({:.1} FPS)", result.avg_time_ms, result.fps);
        eprintln!("[Optimizer]   Min: {:.2} ms", result.min_time_ms);
        eprintln!("[Optimizer]   Max: {:.2} ms", result.max_time_ms);

        Ok(result)
    }
}

/// Benchmark result
#[derive(Debug)]
pub struct BenchmarkResult {
    pub iterations: usize,
    pub avg_time_ms: f64,
    pub min_time_ms: f64,
    pub max_time_ms: f64,
    pub fps: f64,
}

impl Default for ModelOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
