
// Rust代码生成 - 可用的模型URL映射
// 自动从GitHub API获取

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub url: String,
    pub size_mb: f64,
    pub release: String,
}

/// 获取所有可用的预训练模型列表
pub fn get_available_models() -> std::collections::HashMap<String, ModelInfo> {
    let mut models = std::collections::HashMap::new();
    
    models.insert("yolo11n".to_string(), ModelInfo {
        url: "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n.onnx".to_string(),
        size_mb: 10.423833847045898,
        release: "v8.3.0".to_string(),
    });
    
    models.insert("yolo11n-cls".to_string(), ModelInfo {
        url: "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n-cls.onnx".to_string(),
        size_mb: 10.769930839538574,
        release: "v8.3.0".to_string(),
    });
    
    models.insert("yolo11n-obb".to_string(), ModelInfo {
        url: "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n-obb.onnx".to_string(),
        size_mb: 10.493975639343262,
        release: "v8.3.0".to_string(),
    });
    
    models.insert("yolo11n-pose".to_string(), ModelInfo {
        url: "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n-pose.onnx".to_string(),
        size_mb: 11.30433177947998,
        release: "v8.3.0".to_string(),
    });
    
    models.insert("yolo11n-seg".to_string(), ModelInfo {
        url: "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n-seg.onnx".to_string(),
        size_mb: 11.218754768371582,
        release: "v8.3.0".to_string(),
    });
    
    models
}
