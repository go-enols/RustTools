//! 桌面推理性能测试套件
//! 
//! 用于诊断推理帧率为0的问题
//! 
//! 运行方式:
//! cargo test --lib modules::yolo::services::desktop_performance_test -- --nocapture

use std::time::Instant;
use xcap::Monitor;
use image::{DynamicImage, imageops::FilterType};
use tract_onnx::prelude::*;
use ndarray::ArrayViewD;

/// 性能测试结果
#[derive(Debug)]
struct PerformanceResult {
    stage_name: String,
    duration_ms: f64,
}

/// Type alias for tract runnable model
type TractModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// 测试模型加载性能
pub fn test_model_loading(model_path: &str) -> Result<(TractModel, Vec<PerformanceResult>), String> {
    let mut results = Vec::new();
    
    // 测试1: 模型加载时间
    let start = Instant::now();
    let model = tract_onnx::onnx()
        .model_for_path(model_path)
        .map_err(|e| format!("Failed to load model: {}", e))?;
    let load_time = start.elapsed().as_secs_f64() * 1000.0;
    results.push(PerformanceResult {
        stage_name: "Model Loading".to_string(),
        duration_ms: load_time,
    });
    eprintln!("[Performance] Model loading: {:.2}ms", load_time);
    
    // 测试2: 模型编译时间
    let start = Instant::now();
    let model = model
        .with_input_fact(0, f32::fact(&[1, 3, 640, 640]).into())
        .map_err(|e| format!("Failed to configure input: {}", e))?
        .into_typed()
        .map_err(|e| format!("Failed to type model: {}", e))?
        .into_runnable()
        .map_err(|e| format!("Failed to compile model: {}", e))?;
    let compile_time = start.elapsed().as_secs_f64() * 1000.0;
    results.push(PerformanceResult {
        stage_name: "Model Compilation".to_string(),
        duration_ms: compile_time,
    });
    eprintln!("[Performance] Model compilation: {:.2}ms", compile_time);
    
    Ok((model, results))
}

/// 测试图像预处理性能
pub fn test_image_preprocessing(img: &DynamicImage, target_size: usize) -> Result<(Tensor, Vec<PerformanceResult>), String> {
    let mut results = Vec::new();
    
    // 测试resize性能
    let start = Instant::now();
    let resized = img.resize_exact(
        target_size as u32,
        target_size as u32,
        FilterType::Nearest,
    );
    let resize_time = start.elapsed().as_secs_f64() * 1000.0;
    results.push(PerformanceResult {
        stage_name: "Image Resize".to_string(),
        duration_ms: resize_time,
    });
    eprintln!("[Performance] Image resize ({:}x{:}): {:.2}ms", target_size, target_size, resize_time);
    
    // 测试RGB转换性能
    let start = Instant::now();
    let rgb = resized.to_rgb8();
    let rgb_time = start.elapsed().as_secs_f64() * 1000.0;
    results.push(PerformanceResult {
        stage_name: "RGB Conversion".to_string(),
        duration_ms: rgb_time,
    });
    eprintln!("[Performance] RGB conversion: {:.2}ms", rgb_time);
    
    // 测试张量创建性能
    let start = Instant::now();
    let (height, width) = rgb.dimensions();
    let height_usize = height as usize;
    let width_usize = width as usize;
    let mut data = vec![0.0f32; 3 * height_usize * width_usize];
    let pixels = rgb.as_raw();
    
    for i in 0..(height_usize * width_usize) {
        let src_idx = i * 3;
        data[i] = pixels[src_idx + 2] as f32 / 255.0;
        data[(height_usize * width_usize) + i] = pixels[src_idx + 1] as f32 / 255.0;
        data[2 * (height_usize * width_usize) + i] = pixels[src_idx] as f32 / 255.0;
    }
    
    let tensor = Tensor::from_shape(&[1, 3, height_usize, width_usize], &data)
        .map_err(|e| format!("Failed to create tensor: {}", e))?;
    let tensor_time = start.elapsed().as_secs_f64() * 1000.0;
    results.push(PerformanceResult {
        stage_name: "Tensor Creation".to_string(),
        duration_ms: tensor_time,
    });
    eprintln!("[Performance] Tensor creation: {:.2}ms", tensor_time);
    
    Ok((tensor, results))
}

/// 测试推理性能
pub fn test_inference_performance(model: &TractModel, input: Tensor) -> Result<(Vec<PerformanceResult>, Vec<f64>), String> {
    let mut results = Vec::new();
    
    // 预热推理(第一次推理通常较慢)
    eprintln!("[Performance] Warming up inference...");
    let warmup_start = Instant::now();
    let _ = model.run(tvec![input.clone().into()]);
    let warmup_time = warmup_start.elapsed().as_secs_f64() * 1000.0;
    eprintln!("[Performance] Warmup inference: {:.2}ms", warmup_time);
    
    // 多次推理取平均
    let num_iterations = 10;
    let mut inference_times = Vec::new();
    
    eprintln!("[Performance] Running {} inference iterations...", num_iterations);
    for i in 0..num_iterations {
        let start = Instant::now();
        let result = model.run(tvec![input.clone().into()])
            .map_err(|e| format!("Inference failed: {}", e))?;
        let inference_time = start.elapsed().as_secs_f64() * 1000.0;
        inference_times.push(inference_time);
        
        if i == 0 {
            // 分析第一次推理的输出
            let output = &result[0];
            let shape = output.shape();
            eprintln!("[Performance] Output shape: {:?}", shape);
            
            // 获取输出维度
            if shape.len() == 3 {
                let batch_size = shape[0];
                let num_features = shape[1];
                let num_boxes = shape[2];
                eprintln!("[Performance] Batch: {}, Features: {}, Boxes: {}", batch_size, num_features, num_boxes);
                
                // 提取原始输出数据用于分析 - 需要clone因为into_array会消费所有权
                if let Ok(output_data) = output.to_array_view::<f32>() {
                    // 分析输出数据的统计信息
                    analyze_output_data(&output_data, num_features, num_boxes);
                }
            }
        }
        
        eprintln!("[Performance] Inference {}: {:.2}ms", i + 1, inference_time);
    }
    
    // 计算平均推理时间
    let avg_inference_time: f64 = inference_times.iter().sum::<f64>() / num_iterations as f64;
    let min_inference_time = inference_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_inference_time = inference_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    results.push(PerformanceResult {
        stage_name: "Inference (avg)".to_string(),
        duration_ms: avg_inference_time,
    });
    results.push(PerformanceResult {
        stage_name: "Inference (min)".to_string(),
        duration_ms: min_inference_time,
    });
    results.push(PerformanceResult {
        stage_name: "Inference (max)".to_string(),
        duration_ms: max_inference_time,
    });
    
    eprintln!("[Performance] Average inference time: {:.2}ms", avg_inference_time);
    eprintln!("[Performance] Min inference time: {:.2}ms", min_inference_time);
    eprintln!("[Performance] Max inference time: {:.2}ms", max_inference_time);
    eprintln!("[Performance] Theoretical FPS: {:.2}", 1000.0 / avg_inference_time);
    
    Ok((results, inference_times))
}

/// 分析模型输出数据
fn analyze_output_data(output_data: &ArrayViewD<f32>, num_features: usize, num_boxes: usize) {
    eprintln!("\n=== Output Data Analysis ===");
    
    // 分析边界框坐标
    let mut bbox_min = f32::INFINITY;
    let mut bbox_max = f32::NEG_INFINITY;
    let mut bbox_sum = 0.0f32;
    let mut bbox_count = 0usize;
    
    // 只分析前100个框
    for i in 0..num_boxes.min(100) {
        for c in 0..4 {  // 4个bbox坐标
            if let Some(&val) = output_data.get([0, c, i]) {
                bbox_min = bbox_min.min(val);
                bbox_max = bbox_max.max(val);
                bbox_sum += val;
                bbox_count += 1;
            }
        }
    }
    
    if bbox_count > 0 {
        eprintln!("BBox Coordinates (first 100 boxes):");
        eprintln!("  Min: {:.4}, Max: {:.4}, Avg: {:.4}", 
            bbox_min, bbox_max, bbox_sum / bbox_count as f32);
        
        // 判断是否是绝对坐标还是归一化坐标
        if bbox_max > 640.0 {
            eprintln!("  ⚠️ BBox values > 640, likely normalized coordinates");
        } else if bbox_max <= 640.0 {
            eprintln!("  ✓ BBox values <= 640, likely absolute pixel coordinates");
        }
    }
    
    // 分析类别分数
    let num_classes = if num_features > 4 { num_features - 4 } else { 0 };
    if num_classes > 0 {
        let mut class_min = f32::INFINITY;
        let mut class_max = f32::NEG_INFINITY;
        let mut class_sum = 0.0f32;
        let mut class_count = 0usize;
        
        // 只分析前100个框和前10个类别
        for i in 0..num_boxes.min(100) {
            for c in 0..num_classes.min(10) {
                if let Some(&val) = output_data.get([0, c + 4, i]) {
                    class_min = class_min.min(val);
                    class_max = class_max.max(val);
                    class_sum += val;
                    class_count += 1;
                }
            }
        }
        
        if class_count > 0 {
            eprintln!("Class Scores (first 100 boxes, first 10 classes):");
            eprintln!("  Min: {:.4}, Max: {:.4}, Avg: {:.4}", 
                class_min, class_max, class_sum / class_count as f32);
            
            // 判断是否是sigmoid后的概率还是logits
            if class_max <= 1.0 && class_min >= 0.0 {
                eprintln!("  ✓ Values in [0, 1], likely sigmoid probabilities");
            } else if class_max > 1.0 || class_min < 0.0 {
                eprintln!("  ⚠️ Values outside [0, 1], likely raw logits (need sigmoid)");
            }
        }
        
        eprintln!("Detected {} classes in model", num_classes);
    }
    
    // 打印前几个框的原始数据
    eprintln!("\nFirst 5 detection boxes (raw values):");
    for i in 0..5 {
        let cx = *output_data.get([0, 0, i]).unwrap_or(&0.0);
        let cy = *output_data.get([0, 1, i]).unwrap_or(&0.0);
        let w = *output_data.get([0, 2, i]).unwrap_or(&0.0);
        let h = *output_data.get([0, 3, i]).unwrap_or(&0.0);
        
        // 找最大类别分数
        let mut max_score = 0.0f32;
        let mut max_class = 0usize;
        for c in 0..num_classes {
            if let Some(&score) = output_data.get([0, c + 4, i]) {
                if score > max_score {
                    max_score = score;
                    max_class = c;
                }
            }
        }
        
        eprintln!("  Box {}: cx={:.2}, cy={:.2}, w={:.2}, h={:.2}, class={}, score={:.4}", 
            i, cx, cy, w, h, max_class, max_score);
    }
    
    eprintln!("=== End Analysis ===\n");
}

/// 测试完整推理pipeline性能
pub fn test_full_pipeline_performance(model_path: &str, num_frames: usize) -> Result<Vec<PerformanceResult>, String> {
    let mut all_results = Vec::new();
    
    eprintln!("\n========================================");
    eprintln!("Full Pipeline Performance Test");
    eprintln!("========================================\n");
    
    // 1. 测试模型加载
    let (model, load_results) = test_model_loading(model_path)?;
    all_results.extend(load_results);
    
    // 2. 捕获测试图像
    let monitors = Monitor::all().map_err(|e| format!("Failed to get monitors: {}", e))?;
    if monitors.is_empty() {
        return Err("No monitors found".to_string());
    }
    
    let capture_start = Instant::now();
    let captured = monitors[0].capture_image()
        .map_err(|e| format!("Failed to capture screen: {}", e))?;
    let capture_time = capture_start.elapsed().as_secs_f64() * 1000.0;
    all_results.push(PerformanceResult {
        stage_name: "Screen Capture".to_string(),
        duration_ms: capture_time,
    });
    eprintln!("[Performance] Screen capture: {:.2}ms", capture_time);
    eprintln!("[Performance] Captured image size: {}x{}", captured.width(), captured.height());
    
    let orig_img = DynamicImage::ImageRgba8(captured);
    
    // 3. 测试图像预处理
    let (tensor, preprocess_results) = test_image_preprocessing(&orig_img, 640)?;
    all_results.extend(preprocess_results);
    
    // 4. 测试推理性能
    let (inference_results, _inference_times) = test_inference_performance(&model, tensor)?;
    all_results.extend(inference_results);
    
    // 5. 运行完整pipeline多次测试
    eprintln!("\n========================================");
    eprintln!("Full Pipeline Test ({} iterations)", num_frames);
    eprintln!("========================================\n");
    
    let mut full_pipeline_times = Vec::new();
    
    for i in 0..num_frames {
        let frame_start = Instant::now();
        
        // 捕获
        let captured = monitors[0].capture_image().unwrap();
        let img = DynamicImage::ImageRgba8(captured);
        
        // 预处理
        let resized = img.resize_exact(640, 640, FilterType::Nearest);
        let rgb = resized.to_rgb8();
        let (height, width) = rgb.dimensions();
        let height_usize = height as usize;
        let width_usize = width as usize;
        let mut data = vec![0.0f32; 3 * height_usize * width_usize];
        let pixels = rgb.as_raw();
        
        for j in 0..(height_usize * width_usize) {
            let src_idx = j * 3;
            data[j] = pixels[src_idx + 2] as f32 / 255.0;
            data[(height_usize * width_usize) + j] = pixels[src_idx + 1] as f32 / 255.0;
            data[2 * (height_usize * width_usize) + j] = pixels[src_idx] as f32 / 255.0;
        }
        
        let tensor = Tensor::from_shape(&[1, 3, height_usize, width_usize], &data).unwrap();
        
        // 推理
        let _result = model.run(tvec![tensor.into()]).unwrap();
        
        let frame_time = frame_start.elapsed().as_secs_f64() * 1000.0;
        full_pipeline_times.push(frame_time);
        
        if i % 5 == 0 || i == num_frames - 1 {
            eprintln!("[Performance] Frame {}: {:.2}ms ({:.1} FPS)", 
                i + 1, frame_time, 1000.0 / frame_time);
        }
    }
    
    // 计算整体性能
    let avg_frame_time: f64 = full_pipeline_times.iter().sum::<f64>() / num_frames as f64;
    let min_frame_time = full_pipeline_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_frame_time = full_pipeline_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    all_results.push(PerformanceResult {
        stage_name: format!("Full Pipeline (avg, {} frames)", num_frames),
        duration_ms: avg_frame_time,
    });
    all_results.push(PerformanceResult {
        stage_name: "Full Pipeline (min)".to_string(),
        duration_ms: min_frame_time,
    });
    all_results.push(PerformanceResult {
        stage_name: "Full Pipeline (max)".to_string(),
        duration_ms: max_frame_time,
    });
    
    eprintln!("\n========================================");
    eprintln!("Performance Summary");
    eprintln!("========================================");
    eprintln!("Average frame time: {:.2}ms", avg_frame_time);
    eprintln!("Min frame time: {:.2}ms", min_frame_time);
    eprintln!("Max frame time: {:.2}ms", max_frame_time);
    eprintln!("Average FPS: {:.2}", 1000.0 / avg_frame_time);
    eprintln!("\nDetailed Breakdown:");
    
    for result in &all_results {
        eprintln!("  {:.<50} {:.2}ms", result.stage_name, result.duration_ms);
    }
    
    Ok(all_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_analysis() {
        // 这个测试会运行性能分析
        let model_path = "C:\\Users\\25751\\Desktop\\african-wildlife\\yolo11n.onnx";
        
        eprintln!("\n\n========================================");
        eprintln!("Starting Performance Test");
        eprintln!("========================================\n");
        
        match test_full_pipeline_performance(model_path, 10) {
            Ok(_results) => {
                eprintln!("\n✅ Performance test completed successfully");
            }
            Err(e) => {
                eprintln!("\n❌ Performance test failed: {}", e);
                panic!("Performance test failed: {}", e);
            }
        }
    }
}
