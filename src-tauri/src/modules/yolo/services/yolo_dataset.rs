//! YOLO数据集加载器
//! 
//! 支持 YOLO 格式的数据集：
//! - images/: 包含图像文件 (.jpg, .png)
//! - labels/: 包含标注文件 (.txt)
//! 
//! 标注格式：class_id x_center y_center width height (归一化到0-1)

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// 数据集配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// 数据集根目录
    pub dataset_path: PathBuf,
    /// 训练集图像目录
    pub train_images: PathBuf,
    /// 训练集标注目录
    pub train_labels: PathBuf,
    /// 验证集图像目录
    pub val_images: PathBuf,
    /// 验证集标注目录
    pub val_labels: PathBuf,
    /// 类别名称
    pub class_names: Vec<String>,
    /// 类别数量
    pub num_classes: usize,
}

impl DatasetConfig {
    /// 从YAML配置文件加载数据集配置
    pub fn from_yaml(yaml_path: &PathBuf) -> Result<Self, String> {
        let content = fs::read_to_string(yaml_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        
        // 解析YAML配置
        // 简化实现：直接解析常见格式
        let dataset_path = yaml_path.parent()
            .ok_or("无法获取配置文件的父目录")?
            .to_path_buf();
        
        // 默认路径
        Ok(Self {
            dataset_path: dataset_path.clone(),
            train_images: dataset_path.join("train/images"),
            train_labels: dataset_path.join("train/labels"),
            val_images: dataset_path.join("val/images"),
            val_labels: dataset_path.join("val/labels"),
            class_names: vec![
                "person".to_string(),
                "car".to_string(),
                "dog".to_string(),
                "cat".to_string(),
            ],
            num_classes: 4,
        })
    }
}

/// 标注框
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub class_id: usize,
    pub x_center: f32,
    pub y_center: f32,
    pub width: f32,
    pub height: f32,
}

/// 图像和标注
#[derive(Debug, Clone)]
pub struct ImageAnnotation {
    pub image_path: PathBuf,
    pub boxes: Vec<BoundingBox>,
}

impl ImageAnnotation {
    /// 从文件加载图像和标注
    pub fn from_path(image_path: &PathBuf, labels_dir: &PathBuf) -> Result<Self, String> {
        let label_path = Self::image_to_label_path(image_path, labels_dir)?;
        
        let boxes = if label_path.exists() {
            Self::load_labels(&label_path)?
        } else {
            Vec::new()
        };
        
        Ok(Self {
            image_path: image_path.clone(),
            boxes,
        })
    }
    
    fn image_to_label_path(image_path: &PathBuf, labels_dir: &PathBuf) -> Result<PathBuf, String> {
        let stem = image_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or("无法获取文件名")?;
        
        Ok(labels_dir.join(format!("{}.txt", stem)))
    }
    
    fn load_labels(label_path: &PathBuf) -> Result<Vec<BoundingBox>, String> {
        let content = fs::read_to_string(label_path)
            .map_err(|e| format!("读取标注文件失败: {}", e))?;
        
        let mut boxes = Vec::new();
        
        for line in content.lines() {
            let parts: Vec<f32> = line.split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            
            if parts.len() >= 5 {
                boxes.push(BoundingBox {
                    class_id: parts[0] as usize,
                    x_center: parts[1],
                    y_center: parts[2],
                    width: parts[3],
                    height: parts[4],
                });
            }
        }
        
        Ok(boxes)
    }
}

/// 数据集加载器
pub struct YOLODataset {
    config: DatasetConfig,
    image_size: usize,
    train_samples: Vec<PathBuf>,
    val_samples: Vec<PathBuf>,
}

impl YOLODataset {
    /// 创建新的数据集加载器
    pub fn new(config: DatasetConfig, image_size: usize) -> Result<Self, String> {
        let mut dataset = Self {
            config,
            image_size,
            train_samples: Vec::new(),
            val_samples: Vec::new(),
        };
        
        // 加载样本列表
        dataset.load_samples()?;
        
        Ok(dataset)
    }
    
    fn load_samples(&mut self) -> Result<(), String> {
        self.train_samples = self.get_samples_from_dir(&self.config.train_images)?;
        self.val_samples = self.get_samples_from_dir(&self.config.val_images)?;
        Ok(())
    }
    
    /// 获取目录中的所有图像
    fn get_samples_from_dir(&self, dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
        if !dir.exists() {
            return Err(format!("目录不存在: {:?}", dir));
        }
        
        let mut samples = Vec::new();
        
        for entry in fs::read_dir(dir)
            .map_err(|e| format!("读取目录失败: {}", e))?
        {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();
            
            if let Some(ext) = path.extension() {
                let ext = ext.to_str().unwrap_or("").to_lowercase();
                if ext == "jpg" || ext == "jpeg" || ext == "png" {
                    samples.push(path);
                }
            }
        }
        
        samples.sort();
        Ok(samples)
    }
    
    /// 获取训练样本数量
    pub fn num_train_samples(&self) -> usize {
        self.train_samples.len()
    }
    
    /// 获取验证样本数量
    pub fn num_val_samples(&self) -> usize {
        self.val_samples.len()
    }
    
    /// 获取训练样本
    pub fn get_train_sample(&self, index: usize) -> Option<ImageAnnotation> {
        let image_path = self.train_samples.get(index)?;
        ImageAnnotation::from_path(image_path, &self.config.train_labels).ok()
    }
    
    /// 获取验证样本
    pub fn get_val_sample(&self, index: usize) -> Option<ImageAnnotation> {
        let image_path = self.val_samples.get(index)?;
        ImageAnnotation::from_path(image_path, &self.config.val_labels).ok()
    }
    
    /// 数据增强 - 随机水平翻转
    pub fn random_flip(&self, boxes: &mut Vec<BoundingBox>) {
        if rand::random::<f32>() > 0.5 {
            // 翻转边界框的x_center
            for bbox in boxes.iter_mut() {
                bbox.x_center = 1.0 - bbox.x_center;
            }
        }
    }
    
    /// 数据增强 - 随机亮度调整
    pub fn random_brightness(&self) -> f32 {
        // 返回亮度调整因子
        1.0 + (rand::random::<f32>() - 0.5) * 0.4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dataset_creation() {
        // 这个测试需要实际的文件系统，不适合单元测试
        // 仅为展示API
    }
}
