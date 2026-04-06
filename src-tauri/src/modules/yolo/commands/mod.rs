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
    let project_yaml_path = path.join("project.yaml");
    let data_yaml_path = path.join("data.yaml");

    // Check if project directory exists
    if !path.exists() {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some("项目目录不存在".to_string()),
        });
    }

    // Try project.yaml first, then data.yaml
    let (yaml_content, is_data_yaml) = if project_yaml_path.exists() {
        (
            fs::read_to_string(&project_yaml_path)
                .map_err(|e| format!("读取project.yaml失败: {}", e))?,
            false,
        )
    } else if data_yaml_path.exists() {
        (
            fs::read_to_string(&data_yaml_path)
                .map_err(|e| format!("读取data.yaml失败: {}", e))?,
            true,
        )
    } else {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some("不是有效的YOLO项目目录（缺少project.yaml或data.yaml）".to_string()),
        });
    };

    // Parse the YAML to get dataset info
    let dataset_info = parse_dataset_yaml(&yaml_content, &path)?;

    // Create project config
    let project_config = ProjectConfig {
        name: dataset_info.name.clone(),
        path: path.to_string_lossy().to_string(),
        yolo_version: dataset_info.yolo_version.clone(),
        classes: dataset_info.classes.clone(),
        train_split: dataset_info.train_split,
        val_split: dataset_info.val_split,
        image_size: dataset_info.image_size,
        description: Some(format!("项目路径: {}", project_path)),
        images: DatasetPaths {
            train: "images/train".to_string(),
            val: "images/val".to_string(),
        },
        labels: DatasetPaths {
            train: "labels/train".to_string(),
            val: "labels/val".to_string(),
        },
    };

    // If opened from data.yaml, create project.yaml
    if is_data_yaml {
        let yaml_out_content = format!(
            r#"name: {}
yolo_version: {}
description: 项目路径: {}

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
            project_config.name,
            project_config.yolo_version,
            project_path,
            project_config.classes.iter().map(|c| format!("  - {}", c)).collect::<Vec<_>>().join("\n"),
            project_config.train_split,
            project_config.val_split,
            project_config.image_size,
        );

        fs::write(&project_yaml_path, yaml_out_content)
            .map_err(|e| format!("保存project.yaml失败: {}", e))?;
    }

    Ok(ProjectResponse {
        success: true,
        data: Some(project_config),
        error: None,
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

pub mod train;

/// Import an existing YOLO dataset and create a project
#[tauri::command]
pub async fn import_dataset(dataset_path: String) -> Result<ProjectResponse, String> {
    let path = PathBuf::from(&dataset_path);

    // Check if dataset directory exists
    if !path.exists() {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some("数据集目录不存在".to_string()),
        });
    }

    // Try to read data.yaml first (ultralytics standard format)
    let data_yaml_path = path.join("data.yaml");
    let project_yaml_path = path.join("project.yaml");

    // Parse the YAML file
    let yaml_content = if data_yaml_path.exists() {
        fs::read_to_string(&data_yaml_path)
            .map_err(|e| format!("读取data.yaml失败: {}", e))?
    } else if project_yaml_path.exists() {
        fs::read_to_string(&project_yaml_path)
            .map_err(|e| format!("读取project.yaml失败: {}", e))?
    } else {
        return Ok(ProjectResponse {
            success: false,
            data: None,
            error: Some("未找到data.yaml或project.yaml配置文件".to_string()),
        });
    };

    // Parse the YAML to get dataset info
    let dataset_info = parse_dataset_yaml(&yaml_content, &path)?;

    // Create project.yaml from the dataset
    let project_config = ProjectConfig {
        name: dataset_info.name,
        path: path.to_string_lossy().to_string(),
        yolo_version: dataset_info.yolo_version,
        classes: dataset_info.classes,
        train_split: dataset_info.train_split,
        val_split: dataset_info.val_split,
        image_size: dataset_info.image_size,
        description: Some(format!("从 {} 导入的数据集", dataset_path)),
        images: DatasetPaths {
            train: "images/train".to_string(),
            val: "images/val".to_string(),
        },
        labels: DatasetPaths {
            train: "labels/train".to_string(),
            val: "labels/val".to_string(),
        },
    };

    // Save project.yaml
    let yaml_out_content = format!(
        r#"name: {}
yolo_version: {}
description: 从 {} 导入的数据集

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
        project_config.name,
        project_config.yolo_version,
        dataset_path,
        project_config.classes.iter().map(|c| format!("  - {}", c)).collect::<Vec<_>>().join("\n"),
        project_config.train_split,
        project_config.val_split,
        project_config.image_size,
    );

    fs::write(&project_yaml_path, yaml_out_content)
        .map_err(|e| format!("保存project.yaml失败: {}", e))?;

    Ok(ProjectResponse {
        success: true,
        data: Some(project_config),
        error: None,
    })
}

struct DatasetInfo {
    name: String,
    yolo_version: String,
    classes: Vec<String>,
    train_split: f64,
    val_split: f64,
    image_size: i32,
}

fn parse_dataset_yaml(content: &str, project_path: &PathBuf) -> Result<DatasetInfo, String> {
    let mut name = String::new();
    let mut yolo_version = String::from("yolo11");
    let mut classes = Vec::new();
    let mut train_split = 0.8;
    let mut val_split = 0.2;
    let mut image_size = 640;
    let mut path_prefix = String::new();

    // For ultralytics data.yaml format
    let mut in_names = false;

    for line in content.lines() {
        let line = line.trim();

        // Track section state for 'names:'
        if line == "names:" {
            in_names = true;
            continue;
        } else if in_names && line.ends_with(":") && !line.starts_with(" ") && !line.is_empty() {
            // Another top-level section
            in_names = false;
        }

        // Parse names section (ultralytics format with numeric keys)
        if in_names {
            // e.g., "  0: buffalo"
            if line.contains(":") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let class_name = parts[1].trim();
                    if !class_name.is_empty() {
                        classes.push(class_name.to_string());
                    }
                }
            }
            continue;
        }

        // General fields
        if line.starts_with("path:") {
            path_prefix = line.replace("path:", "").trim().to_string();
        } else if line.starts_with("name:") {
            name = line.replace("name:", "").trim().to_string();
        } else if line.starts_with("yolo_version:") {
            yolo_version = line.replace("yolo_version:", "").trim().to_string();
        } else if line.starts_with("- ") && !line.contains(":") {
            // Class entry in project.yaml format
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

    // If name is empty, use the folder name
    if name.is_empty() {
        name = project_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "imported_dataset".to_string());
    }

    // If no classes found, try to infer from labels folder
    if classes.is_empty() {
        let labels_train = project_path.join("labels/train");
        if labels_train.exists() {
            // Try to read first label file to count classes
            if let Ok(entries) = fs::read_dir(&labels_train) {
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let mut max_class_id = 0;
                        for line in content.lines() {
                            let parts: Vec<&str> = line.trim().split_whitespace().collect();
                            if let Some(first) = parts.first() {
                                if let Ok(id) = first.parse::<usize>() {
                                    max_class_id = max_class_id.max(id);
                                }
                            }
                        }
                        // Assume class IDs are 0-indexed and consecutive
                        for i in 0..=max_class_id {
                            classes.push(format!("class_{}", i));
                        }
                        break;
                    }
                }
            }
        }
    }

    // If still no classes, provide default
    if classes.is_empty() {
        classes.push("object".to_string());
    }

    Ok(DatasetInfo {
        name,
        yolo_version,
        classes,
        train_split,
        val_split,
        image_size,
    })
}
