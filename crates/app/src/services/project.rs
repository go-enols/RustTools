use std::fs;
use std::path::PathBuf;

use crate::models::{DatasetPaths, ProjectConfig, ProjectResponse};
use serde_yaml::Value;

/// Create a new YOLO project with default folder structure
pub fn create_project(config: ProjectConfig) -> ProjectResponse {
    let project_path = PathBuf::from(&config.path);

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
            return ProjectResponse::err(format!("Failed to create directory {}: {}", dir, e));
        }
    }

    // Save project config to YAML
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
        yaml_quote(&config.name),
        yaml_quote(&config.yolo_version),
        yaml_quote(config.description.as_deref().unwrap_or("")),
        config.classes.iter().map(|c| format!("  - {}", yaml_quote(c))).collect::<Vec<_>>().join("\n"),
        config.train_split,
        config.val_split,
        config.image_size,
    );

    if let Err(e) = fs::write(&config_path, yaml_content) {
        return ProjectResponse::err(format!("Failed to save project config: {}", e));
    }

    // Create data.yaml for YOLO training
    let data_yaml_path = project_path.join("data.yaml");
    let data_yaml_content = format!(
        r#"# YOLO Dataset Configuration
path: {}
train: images/train
val: images/val

names:
{}
"#,
        yaml_quote(&project_path.to_string_lossy().replace('\\', "/")),
        config.classes.iter().enumerate().map(|(i, c)| format!("  {}: {}", i, yaml_quote(c))).collect::<Vec<_>>().join("\n"),
    );

    if let Err(e) = fs::write(&data_yaml_path, data_yaml_content) {
        return ProjectResponse::err(format!("Failed to save data.yaml: {}", e));
    }

    ProjectResponse::ok(ProjectConfig {
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
    })
}

/// Open an existing YOLO project
pub fn open_project(project_path: String) -> ProjectResponse {
    let path = PathBuf::from(&project_path);
    let project_yaml_path = path.join("project.yaml");
    let data_yaml_path = path.join("data.yaml");
    let dataset_yaml_path = path.join("dataset.yaml");

    if !path.exists() {
        return ProjectResponse::err("Project directory does not exist");
    }

    let (yaml_content, is_data_yaml) = if project_yaml_path.exists() {
        match fs::read_to_string(&project_yaml_path) {
            Ok(c) => (c, false),
            Err(e) => return ProjectResponse::err(format!("Failed to read project.yaml: {}", e)),
        }
    } else if data_yaml_path.exists() {
        match fs::read_to_string(&data_yaml_path) {
            Ok(c) => (c, true),
            Err(e) => return ProjectResponse::err(format!("Failed to read data.yaml: {}", e)),
        }
    } else if dataset_yaml_path.exists() {
        match fs::read_to_string(&dataset_yaml_path) {
            Ok(c) => (c, true),
            Err(e) => return ProjectResponse::err(format!("Failed to read dataset.yaml: {}", e)),
        }
    } else {
        return ProjectResponse::err("Not a valid YOLO project (missing project.yaml, data.yaml or dataset.yaml)");
    };

    let dataset_info = match parse_dataset_yaml(&yaml_content, &path) {
        Ok(info) => info,
        Err(e) => return ProjectResponse::err(e),
    };

    let project_config = ProjectConfig {
        name: dataset_info.name.clone(),
        path: path.to_string_lossy().to_string(),
        yolo_version: dataset_info.yolo_version.clone(),
        classes: dataset_info.classes.clone(),
        train_split: dataset_info.train_split,
        val_split: dataset_info.val_split,
        image_size: dataset_info.image_size,
        description: Some(format!("Project path: {}", project_path)),
        images: dataset_info.images.clone(),
        labels: dataset_info.labels.clone(),
    };

    if is_data_yaml {
        let yaml_out = format!(
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
            yaml_quote(&project_config.name),
            yaml_quote(&project_config.yolo_version),
            yaml_quote(&format!("Project path: {}", project_path)),
            project_config.classes.iter().map(|c| format!("  - {}", yaml_quote(c))).collect::<Vec<_>>().join("\n"),
            project_config.train_split,
            project_config.val_split,
            project_config.image_size,
        );
        let _ = fs::write(&project_yaml_path, yaml_out);
    }

    // Regenerate data.yaml
    let data_yaml_content = format!(
        r#"# YOLO Dataset Configuration
path: {}
train: images/train
val: images/val

names:
{}
"#,
        yaml_quote(&path.to_string_lossy().replace('\\', "/")),
        project_config.classes.iter().enumerate().map(|(i, c)| format!("  {}: {}", i, yaml_quote(c))).collect::<Vec<_>>().join("\n"),
    );
    let _ = fs::write(&data_yaml_path, data_yaml_content);

    ProjectResponse::ok(project_config)
}

struct DatasetInfo {
    name: String,
    yolo_version: String,
    classes: Vec<String>,
    train_split: f64,
    val_split: f64,
    image_size: i32,
    images: DatasetPaths,
    labels: DatasetPaths,
}

fn parse_dataset_yaml(content: &str, project_path: &PathBuf) -> Result<DatasetInfo, String> {
    use serde_yaml::Value;

    let root: Value = serde_yaml::from_str(content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;

    let mut name = String::new();
    let mut yolo_version = String::from("yolo11");
    let mut classes = Vec::new();
    let mut train_split = 0.8;
    let mut val_split = 0.2;
    let mut image_size = 640;
    let mut images_train = String::from("images/train");
    let mut images_val = String::from("images/val");
    let mut labels_train = String::from("labels/train");
    let mut labels_val = String::from("labels/val");

    if let Value::Mapping(map) = root {
        // name
        if let Some(Value::String(v)) = map.get(&Value::String("name".to_string())) {
            name = v.clone();
        }

        // yolo_version
        if let Some(v) = map.get(&Value::String("yolo_version".to_string())) {
            yolo_version = yaml_value_to_string(v);
        }

        // image_size
        if let Some(v) = map.get(&Value::String("image_size".to_string())) {
            if let Ok(n) = yaml_value_to_string(v).parse::<i32>() {
                image_size = n;
            }
        }

        // train_split / val_split
        if let Some(v) = map.get(&Value::String("train_split".to_string())) {
            if let Ok(n) = yaml_value_to_string(v).parse::<f64>() {
                train_split = n;
            }
        }
        if let Some(v) = map.get(&Value::String("val_split".to_string())) {
            if let Ok(n) = yaml_value_to_string(v).parse::<f64>() {
                val_split = n;
            }
        }

        // images paths
        if let Some(Value::Mapping(img_map)) = map.get(&Value::String("images".to_string())) {
            if let Some(v) = img_map.get(&Value::String("train".to_string())) {
                images_train = yaml_value_to_string(v);
            }
            if let Some(v) = img_map.get(&Value::String("val".to_string())) {
                images_val = yaml_value_to_string(v);
            }
        } else if let Some(v) = map.get(&Value::String("train".to_string())) {
            // flat format: train: images/train
            let s = yaml_value_to_string(v);
            if !s.is_empty() && !s.eq_ignore_ascii_case("null") {
                images_train = s;
            }
        }
        if let Some(v) = map.get(&Value::String("val".to_string())) {
            let s = yaml_value_to_string(v);
            if !s.is_empty() && !s.eq_ignore_ascii_case("null") {
                images_val = s;
            }
        }

        // labels paths
        if let Some(Value::Mapping(lbl_map)) = map.get(&Value::String("labels".to_string())) {
            if let Some(v) = lbl_map.get(&Value::String("train".to_string())) {
                labels_train = yaml_value_to_string(v);
            }
            if let Some(v) = lbl_map.get(&Value::String("val".to_string())) {
                labels_val = yaml_value_to_string(v);
            }
        }

        // classes / names
        if let Some(v) = map.get(&Value::String("classes".to_string())) {
            classes = extract_classes(v);
        }
        if classes.is_empty() {
            if let Some(v) = map.get(&Value::String("names".to_string())) {
                classes = extract_classes(v);
            }
        }

        // nc (number of classes) - validate but we already got names
        if let Some(v) = map.get(&Value::String("nc".to_string())) {
            if let Ok(_nc) = yaml_value_to_string(v).parse::<usize>() {
                // nc just validates class count; names already extracted
            }
        }
    }

    if name.is_empty() {
        name = project_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "imported_dataset".to_string());
    }

    // Fallback: infer classes from label files if still empty
    if classes.is_empty() {
        let labels_train_dir = project_path.join(&labels_train);
        let labels_dir = if labels_train_dir.exists() {
            labels_train_dir
        } else {
            project_path.join("labels/train")
        };
        if labels_dir.exists() {
            if let Ok(entries) = fs::read_dir(&labels_dir) {
                let mut max_class_id: Option<usize> = None;
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        for line in content.lines() {
                            let parts: Vec<&str> = line.trim().split_whitespace().collect();
                            if let Some(first) = parts.first() {
                                if let Ok(id) = first.parse::<usize>() {
                                    max_class_id = Some(max_class_id.unwrap_or(0).max(id));
                                }
                            }
                        }
                    }
                }
                if let Some(max_id) = max_class_id {
                    for i in 0..=max_id {
                        classes.push(format!("class_{}", i));
                    }
                }
            }
        }
    }

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

fn yaml_value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// 将字符串安全地转为 YAML 标量值（需要时自动加引号/转义）
fn yaml_quote(s: &str) -> String {
    serde_yaml::to_string(&serde_yaml::Value::String(s.to_string()))
        .unwrap_or_default()
        .trim_end()
        .to_string()
}

fn extract_classes(v: &serde_yaml::Value) -> Vec<String> {
    let mut classes = Vec::new();
    match v {
        // names:
        //   0: person
        //   1: bicycle
        Value::Mapping(m) => {
            let mut entries: Vec<(String, String)> = Vec::new();
            for (k, val) in m.iter() {
                let key = yaml_value_to_string(k);
                let value = yaml_value_to_string(val);
                if !value.is_empty() {
                    entries.push((key, value));
                }
            }
            // Sort by key (numeric if possible) to maintain order
            entries.sort_by(|a, b| {
                let na = a.0.parse::<usize>();
                let nb = b.0.parse::<usize>();
                match (na, nb) {
                    (Ok(a), Ok(b)) => a.cmp(&b),
                    _ => a.0.cmp(&b.0),
                }
            });
            for (_, val) in entries {
                classes.push(val);
            }
        }
        // classes:
        //   - person
        //   - bicycle
        Value::Sequence(seq) => {
            for item in seq {
                let s = yaml_value_to_string(item);
                if !s.is_empty() {
                    classes.push(s);
                }
            }
        }
        _ => {}
    }
    classes
}

/// Update project classes in project.yaml
pub fn update_classes(project_path: String, classes: Vec<String>) -> Result<(), String> {
    let path = PathBuf::from(&project_path);
    let config_path = path.join("project.yaml");

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();
    let mut in_classes = false;
    let mut classes_found = false;

    for line in lines {
        if line.trim() == "classes:" {
            in_classes = true;
            new_lines.push(line.to_string());
            for class in &classes {
                new_lines.push(format!("  - {}", class));
            }
            classes_found = true;
        } else if in_classes && line.starts_with("- ") {
            continue;
        } else if in_classes && !line.trim().starts_with("- ") && !line.trim().is_empty() {
            in_classes = false;
            new_lines.push(line.to_string());
        } else {
            new_lines.push(line.to_string());
        }
    }

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

    fs::write(&config_path, new_lines.join("\n"))
        .map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

/// Scan a project directory and return statistics
pub fn scan_project(project_path: &str) -> crate::models::ProjectScanResult {
    use crate::models::ProjectScanResult;
    let mut result = ProjectScanResult::default();
    let base = std::path::PathBuf::from(project_path);

    // 统计训练图像
    let train_img_dir = base.join("images/train");
    if let Ok(entries) = std::fs::read_dir(&train_img_dir) {
        result.train_images = entries.flatten().filter(|e| {
            e.path().extension().map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp")
            }).unwrap_or(false)
        }).count();
    }

    // 统计验证图像
    let val_img_dir = base.join("images/val");
    if let Ok(entries) = std::fs::read_dir(&val_img_dir) {
        result.val_images = entries.flatten().filter(|e| {
            e.path().extension().map(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp")
            }).unwrap_or(false)
        }).count();
    }

    // 统计标注文件
    let train_label_dir = base.join("labels/train");
    if let Ok(entries) = std::fs::read_dir(&train_label_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                result.total_annotations += content.lines().filter(|l| !l.trim().is_empty()).count();
            }
        }
    }
    let val_label_dir = base.join("labels/val");
    if let Ok(entries) = std::fs::read_dir(&val_label_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                result.total_annotations += content.lines().filter(|l| !l.trim().is_empty()).count();
            }
        }
    }

    // 扫描模型文件
    let models_dir = base.join("models");
    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().eq_ignore_ascii_case("pt") {
                    result.model_count += 1;
                    if result.models.len() < 5 {
                        if let Some(name) = path.file_name() {
                            result.models.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    // 扫描训练结果
    let runs_dir = base.join("runs");
    if let Ok(entries) = std::fs::read_dir(&runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.run_count += 1;
                if result.runs.len() < 5 {
                    if let Some(name) = path.file_name() {
                        result.runs.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_yaml_names_dict() {
        let content = r#"
path: /home/user/project
train: images/train
val: images/val
nc: 3
names:
  0: person
  1: car
  2: dog
"#;
        let result = parse_dataset_yaml(content, &PathBuf::from("/home/user/project"));
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "project");
        assert_eq!(info.classes, vec!["person", "car", "dog"]);
        assert_eq!(info.images.train, "images/train");
        assert_eq!(info.images.val, "images/val");
    }

    #[test]
    fn test_parse_project_yaml_classes_list() {
        let content = r#"
name: myproject
yolo_version: yolo11
description: Test project
classes:
  - person
  - bicycle
  - car
train_split: 0.8
val_split: 0.2
image_size: 640
images:
  train: images/train
  val: images/val
labels:
  train: labels/train
  val: labels/val
"#;
        let result = parse_dataset_yaml(content, &PathBuf::from("/tmp/myproject"));
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "myproject");
        assert_eq!(info.yolo_version, "yolo11");
        assert_eq!(info.classes, vec!["person", "bicycle", "car"]);
        assert_eq!(info.train_split, 0.8);
        assert_eq!(info.val_split, 0.2);
        assert_eq!(info.image_size, 640);
        assert_eq!(info.images.train, "images/train");
        assert_eq!(info.labels.train, "labels/train");
    }

    #[test]
    fn test_parse_flat_paths() {
        let content = r#"
name: flat_project
train: images/train
val: images/val
classes:
  - object
"#;
        let result = parse_dataset_yaml(content, &PathBuf::from("/tmp/flat"));
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.images.train, "images/train");
        assert_eq!(info.images.val, "images/val");
    }

    #[test]
    fn test_parse_empty_name_fallback() {
        let content = r#"
train: images/train
val: images/val
names:
  0: object
"#;
        let result = parse_dataset_yaml(content, &PathBuf::from("/tmp/test_project"));
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "test_project");
    }

    #[test]
    fn test_parse_names_unordered_keys() {
        let content = r#"
path: /project
names:
  2: zebra
  0: apple
  1: banana
"#;
        let result = parse_dataset_yaml(content, &PathBuf::from("/project"));
        assert!(result.is_ok());
        let info = result.unwrap();
        // Should be sorted by numeric key
        assert_eq!(info.classes, vec!["apple", "banana", "zebra"]);
    }
}
