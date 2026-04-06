use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct YoloAnnotation {
    pub class_id: usize,
    pub x_center: f64,
    pub y_center: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Serialize)]
pub struct AnnotationResponse {
    pub success: bool,
    pub data: Option<Vec<YoloAnnotation>>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub path: String,
    pub yolo_version: String,
    pub classes: Vec<String>,
    pub train_split: f64,
    pub val_split: f64,
    pub image_size: i32,
    pub description: Option<String>,
    #[serde(default)]
    pub images: DatasetPaths,
    #[serde(default)]
    pub labels: DatasetPaths,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DatasetPaths {
    pub train: String,
    pub val: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub success: bool,
    pub data: Option<ProjectConfig>,
    pub error: Option<String>,
}

/// Create a new YOLO project with default folder structure
#[tauri::command]
pub async fn project_create(config: ProjectConfig) -> Result<ProjectResponse, String> {
    let project_path = PathBuf::from(&config.path);

    // Create project directory structure
    let dirs = [
        "images/train",
        "images/val",
        "labels/train",
        "labels/val",
        "models",
        "runs",
        "weights",
    ];

    for dir in &dirs {
        let dir_path = project_path.join(dir);
        if let Err(e) = fs::create_dir_all(&dir_path) {
            return Ok(ProjectResponse {
                success: false,
                data: None,
                error: Some(format!("创建目录失败 {}: {}", dir, e)),
            });
        }
    }

    // Save project config to YAML file
    let config_path = project_path.join("project.yaml");
    let yaml_content = format!(
        r#"name: {}
yolo_version: {}
description: {}

classes:
{}

train_split: {}
val_split: {}
image_size: {}

images:
  train: images/train
  val: images/val

labels:
  train: labels/train
  val: labels/val
"#,
        config.name,
        config.yolo_version,
        config.description.as_deref().unwrap_or(""),
        config.classes.iter().map(|c| format!("  - {}", c)).collect::<Vec<_>>().join("\n"),
        config.train_split,
        config.val_split,
        config.image_size,
    );

    if let Err(e) = fs::write(&config_path, yaml_content) {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some(format!("保存项目配置失败: {}", e)),
        });
    }

    Ok(ProjectResponse {
        success: true,
        data: Some(ProjectConfig {
            name: config.name,
            path: project_path.to_string_lossy().to_string(),
            yolo_version: config.yolo_version,
            classes: config.classes,
            train_split: config.train_split,
            val_split: config.val_split,
            image_size: config.image_size,
            description: config.description,
            images: DatasetPaths {
                train: "images/train".to_string(),
                val: "images/val".to_string(),
            },
            labels: DatasetPaths {
                train: "labels/train".to_string(),
                val: "labels/val".to_string(),
            },
        }),
        error: None,
    })
}

/// Open an existing YOLO project
#[tauri::command]
pub async fn project_open(project_path: String) -> Result<ProjectResponse, String> {
    let path = PathBuf::from(&project_path);
    let config_path = path.join("project.yaml");

    // Check if project directory exists
    if !path.exists() {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some("项目目录不存在".to_string()),
        });
    }

    // Check if config file exists
    if !config_path.exists() {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some("不是有效的YOLO项目目录（缺少project.yaml）".to_string()),
        });
    }

    // Read and parse config file
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("读取项目配置失败: {}", e))?;

    // Simple YAML parsing (in production, use a proper YAML crate)
    let config = parse_project_yaml(&content, &path)?;

    Ok(ProjectResponse {
        success: true,
        data: Some(config),
        error: None,
    })
}

fn parse_project_yaml(content: &str, project_path: &PathBuf) -> Result<ProjectConfig, String> {
    let mut name = String::new();
    let mut yolo_version = String::from("yolo8");
    let mut description = Option::<String>::None;
    let mut classes = Vec::new();
    let mut train_split = 0.8;
    let mut val_split = 0.2;
    let mut image_size = 640;

    // Default dataset paths (used if not found in YAML)
    let mut images_train = String::from("images/train");
    let mut images_val = String::from("images/val");
    let mut labels_train = String::from("labels/train");
    let mut labels_val = String::from("labels/val");

    // Simple state machine for parsing nested sections
    let mut in_images = false;
    let mut in_labels = false;

    for line in content.lines() {
        let line = line.trim();

        // Track section state
        if line == "images:" {
            in_images = true;
            in_labels = false;
            continue;
        } else if line == "labels:" {
            in_labels = true;
            in_images = false;
            continue;
        } else if line.ends_with(":") && !line.contains(" ") {
            // Another top-level section starting
            in_images = false;
            in_labels = false;
            continue;
        }

        // Parse based on current section
        if in_images {
            if line.starts_with("train:") {
                images_train = line.replace("train:", "").trim().to_string();
            } else if line.starts_with("val:") {
                images_val = line.replace("val:", "").trim().to_string();
            }
            continue;
        }
        if in_labels {
            if line.starts_with("train:") {
                labels_train = line.replace("train:", "").trim().to_string();
            } else if line.starts_with("val:") {
                labels_val = line.replace("val:", "").trim().to_string();
            }
            continue;
        }

        // General fields
        if line.starts_with("name:") {
            name = line.replace("name:", "").trim().to_string();
        } else if line.starts_with("yolo_version:") {
            yolo_version = line.replace("yolo_version:", "").trim().to_string();
        } else if line.starts_with("description:") {
            let desc = line.replace("description:", "").trim().to_string();
            if !desc.is_empty() {
                description = Some(desc);
            }
        } else if line.starts_with("- ") && !line.contains(":") {
            // Class entry
            classes.push(line.replace("-", "").trim().to_string());
        } else if line.starts_with("train_split:") {
            if let Ok(val) = line.replace("train_split:", "").trim().parse::<f64>() {
                train_split = val;
            }
        } else if line.starts_with("val_split:") {
            if let Ok(val) = line.replace("val_split:", "").trim().parse::<f64>() {
                val_split = val;
            }
        } else if line.starts_with("image_size:") {
            if let Ok(val) = line.replace("image_size:", "").trim().parse::<i32>() {
                image_size = val;
            }
        }
    }

    if name.is_empty() {
        return Err("项目配置无效：缺少项目名称".to_string());
    }

    Ok(ProjectConfig {
        name,
        path: project_path.to_string_lossy().to_string(),
        yolo_version,
        classes,
        train_split,
        val_split,
        image_size,
        description,
        images: DatasetPaths {
            train: images_train,
            val: images_val,
        },
        labels: DatasetPaths {
            train: labels_train,
            val: labels_val,
        },
    })
}

/// Update project classes in project.yaml
#[tauri::command]
pub async fn update_classes(
    project_path: String,
    classes: Vec<String>,
) -> Result<ProjectResponse, String> {
    let path = PathBuf::from(&project_path);
    let config_path = path.join("project.yaml");

    // Read existing content
    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;

    // Rebuild YAML with updated classes
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();
    let mut in_classes = false;
    let mut classes_found = false;

    for line in lines {
        if line.trim() == "classes:" {
            in_classes = true;
            new_lines.push(line.to_string());
            // Add new classes
            for class in &classes {
                new_lines.push(format!("  - {}", class));
            }
            classes_found = true;
        } else if in_classes && line.starts_with("- ") {
            // Skip old classes, already added new ones
            continue;
        } else if in_classes && !line.trim().starts_with("- ") && !line.trim().is_empty() {
            // Exited classes section
            in_classes = false;
            new_lines.push(line.to_string());
        } else {
            new_lines.push(line.to_string());
        }
    }

    // If no classes section existed, add it after name
    if !classes_found {
        let mut final_lines: Vec<String> = Vec::new();
        for line in new_lines {
            final_lines.push(line.clone());
            if line.starts_with("name:") {
                final_lines.push("classes:".to_string());
                for class in &classes {
                    final_lines.push(format!("  - {}", class));
                }
            }
        }
        new_lines = final_lines;
    }

    let new_content = new_lines.join("\n");

    // Write back
    fs::write(&config_path, new_content)
        .map_err(|e| format!("保存配置文件失败: {}", e))?;

    Ok(ProjectResponse { success: true, data: None, error: None })
}

/// Load annotations from a YOLO label file
#[tauri::command]
pub async fn load_annotation(
    label_path: String,
) -> Result<AnnotationResponse, String> {
    let path = PathBuf::from(&label_path);

    if !path.exists() {
        return Ok(AnnotationResponse {
            success: true,
            data: Some(Vec::new()),
            error: None,
        });
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取标注文件失败: {}", e))?;

    let annotations: Vec<YoloAnnotation> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<f64> = line
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 5 {
                Some(YoloAnnotation {
                    class_id: parts[0] as usize,
                    x_center: parts[1],
                    y_center: parts[2],
                    width: parts[3],
                    height: parts[4],
                })
            } else {
                None
            }
        })
        .collect();

    Ok(AnnotationResponse {
        success: true,
        data: Some(annotations),
        error: None,
    })
}

/// Save annotations to a YOLO label file
#[tauri::command]
pub async fn save_annotation(
    label_path: String,
    annotations: Vec<YoloAnnotation>,
) -> Result<AnnotationResponse, String> {
    let path = PathBuf::from(&label_path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    // Convert to YOLO format (normalized coordinates)
    let content: String = annotations
        .iter()
        .map(|ann| {
            format!(
                "{} {:.6} {:.6} {:.6} {:.6}",
                ann.class_id, ann.x_center, ann.y_center, ann.width, ann.height
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(&path, content)
        .map_err(|e| format!("保存标注文件失败: {}", e))?;

    Ok(AnnotationResponse {
        success: true,
        data: None,
        error: None,
    })
}
