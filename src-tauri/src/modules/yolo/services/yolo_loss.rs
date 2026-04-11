//! YOLO损失函数实现
//! 
//! 包含三种损失：
//! - Box Loss (CIoU Loss): 边界框回归
//! - Classification Loss: 类别预测
//! - Distribution Focal Loss (DFL): 分布焦点损失

use serde::{Deserialize, Serialize};

/// YOLO损失函数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YOLOLossConfig {
    pub box_weight: f32,      // 边界框损失权重
    pub cls_weight: f32,      // 分类损失权重
    pub dfl_weight: f32,      // DFL损失权重
    pub cls_for_bg: f32,      // 背景类别的分类权重
}

impl Default for YOLOLossConfig {
    fn default() -> Self {
        Self {
            box_weight: 7.5,
            cls_weight: 0.5,
            dfl_weight: 1.5,
            cls_for_bg: 0.25,
        }
    }
}

/// 预测结果
#[derive(Debug, Clone)]
pub struct YOLOPrediction {
    pub boxes: Vec<f32>,      // [num_anchors, 4] (x, y, w, h)
    pub objectness: Vec<f32>, // [num_anchors]
    pub class_probs: Vec<f32>, // [num_anchors, num_classes]
}

/// 目标标注
#[derive(Debug, Clone)]
pub struct YOLOTarget {
    pub boxes: Vec<BoxTarget>,
    pub image_id: usize,
}

/// 单个目标框
#[derive(Debug, Clone)]
pub struct BoxTarget {
    pub class_id: usize,
    pub x_center: f32,
    pub y_center: f32,
    pub width: f32,
    pub height: f32,
}

/// YOLO损失函数
pub struct YOLOLoss {
    config: YOLOLossConfig,
    num_classes: usize,
}

impl YOLOLoss {
    pub fn new(config: YOLOLossConfig, num_classes: usize) -> Self {
        Self { config, num_classes }
    }
    
    /// 计算完整的YOLO损失
    pub fn forward(
        &self,
        predictions: &[f32],  // 展平的预测数组
        targets: &[YOLOTarget],
        batch_size: usize,
        num_anchors: usize,
    ) -> YOLOLossOutput {
        // TODO: 实现完整的损失计算
        // 当前是占位实现，返回零损失
        YOLOLossOutput {
            total_loss: 0.0,
            box_loss: 0.0,
            cls_loss: 0.0,
            dfl_loss: 0.0,
        }
    }
}

/// 损失输出
#[derive(Debug, Clone)]
pub struct YOLOLossOutput {
    pub total_loss: f32,
    pub box_loss: f32,
    pub cls_loss: f32,
    pub dfl_loss: f32,
}

/// CIoU Loss 实现
pub fn ciou_loss(
    pred_boxes: &[f32],
    target_boxes: &[f32],
) -> f32 {
    // Distribution Focal Loss
    // 用于细化边界框的离散分布
    // TODO: 实现完整的CIoU计算
    0.0
}

/// 计算IoU
pub fn calculate_iou(
    pred_x: f32,
    pred_y: f32,
    pred_w: f32,
    pred_h: f32,
    target_x: f32,
    target_y: f32,
    target_w: f32,
    target_h: f32,
) -> f32 {
    // 计算交集
    let inter_x1 = (pred_x - pred_w / 2.0).max(target_x - target_w / 2.0);
    let inter_y1 = (pred_y - pred_h / 2.0).max(target_y - target_h / 2.0);
    let inter_x2 = (pred_x + pred_w / 2.0).min(target_x + target_w / 2.0);
    let inter_y2 = (pred_y + pred_h / 2.0).min(target_y + target_h / 2.0);
    
    let inter_area = ((inter_x2 - inter_x1).max(0.0) * 
                     (inter_y2 - inter_y1).max(0.0));
    
    // 计算并集
    let pred_area = pred_w * pred_h;
    let target_area = target_w * target_h;
    let union_area = pred_area + target_area - inter_area;
    
    // IoU = 交集 / 并集
    if union_area > 0.0 {
        inter_area / union_area
    } else {
        0.0
    }
}

///  Focal Loss 用于类别不平衡
pub fn focal_loss(
    predictions: &[f32],
    targets: &[f32],
    alpha: f32,
    gamma: f32,
) -> f32 {
    // TODO: 实现Focal Loss
    // Focal Loss = -alpha * (1 - p_t)^gamma * log(p_t)
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ciou_loss() {
        let pred = vec![0.5, 0.5, 0.2, 0.2];
        let target = vec![0.5, 0.5, 0.2, 0.2];
        let loss = ciou_loss(&pred, &target);
        assert_eq!(loss, 0.0);
    }
    
    #[test]
    fn test_iou_calculation() {
        let iou = calculate_iou(0.5, 0.5, 0.4, 0.4, 0.5, 0.5, 0.4, 0.4);
        assert_eq!(iou, 1.0);
    }
}
