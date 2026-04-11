#!/usr/bin/env python3
"""
获取所有可用的YOLO ONNX模型URL
"""
import requests
import json

def get_all_onnx_models():
    """获取所有可用的ONNX模型"""
    all_models = {}
    
    # 定义要检查的releases
    releases = [
        "v8.4.0",  # YOLO11
        "v8.3.0",  # YOLO11
        "v8.2.0",  # YOLOv8
        "v8.1.0",  # YOLOv8
        "v8.0.0",  # YOLOv8
    ]
    
    print("="*100)
    print("扫描所有GitHub releases中的ONNX模型...")
    print("="*100)
    
    for tag in releases:
        url = f"https://api.github.com/repos/ultralytics/assets/releases/tags/{tag}"
        
        try:
            response = requests.get(url, timeout=30)
            if response.status_code == 200:
                data = response.json()
                assets = data.get('assets', [])
                
                count = 0
                for asset in assets:
                    name = asset['name']
                    if name.endswith('.onnx'):
                        size_mb = asset['size'] / (1024 * 1024)
                        browser_download_url = asset['browser_download_url']
                        
                        # 简化模型名称
                        model_name = name.replace('.onnx', '')
                        all_models[model_name] = {
                            'url': browser_download_url,
                            'size_mb': size_mb,
                            'release': tag
                        }
                        count += 1
                
                print(f"✓ {tag:10} | 找到 {count:3} 个ONNX模型")
            else:
                print(f"✗ {tag:10} | HTTP {response.status_code}")
        except Exception as e:
            print(f"✗ {tag:10} | 错误: {e}")
    
    return all_models

def print_model_list(models):
    """打印模型列表"""
    print("\n" + "="*100)
    print("可用的YOLO模型列表")
    print("="*100)
    
    # 按类型分组
    categories = {
        'detection': [],
        'segmentation': [],
        'pose': [],
        'obb': [],
        'classification': []
    }
    
    for name, info in sorted(models.items()):
        if 'seg' in name:
            categories['segmentation'].append((name, info))
        elif 'pose' in name:
            categories['pose'].append((name, info))
        elif 'obb' in name:
            categories['obb'].append((name, info))
        elif 'cls' in name:
            categories['classification'].append((name, info))
        else:
            categories['detection'].append((name, info))
    
    for cat, items in categories.items():
        if items:
            print(f"\n{cat.upper()}:")
            print("-" * 100)
            for name, info in sorted(items):
                print(f"  {name:30} | {info['size_mb']:7.2f} MB | {info['release']:10} | {info['url']}")
    
    print("\n" + "="*100)
    print(f"总计: {len(models)} 个模型")

def save_to_rust_code(models):
    """生成Rust代码"""
    rust_code = '''
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
    
'''
    
    for name, info in sorted(models.items()):
        rust_code += f'''    models.insert("{name}".to_string(), ModelInfo {{
        url: "{info['url']}".to_string(),
        size_mb: {info['size_mb']},
        release: "{info['release']}".to_string(),
    }});
    
'''
    
    rust_code += '''    models
}
'''
    
    with open('available_models.rs', 'w', encoding='utf-8') as f:
        f.write(rust_code)
    
    print(f"\n✓ Rust代码已保存到 available_models.rs")

def main():
    models = get_all_onnx_models()
    
    if models:
        print_model_list(models)
        save_to_rust_code(models)
        
        # 保存为JSON
        with open('available_models.json', 'w', encoding='utf-8') as f:
            json.dump(models, f, indent=2, ensure_ascii=False)
        print("✓ JSON已保存到 available_models.json")
    else:
        print("\n✗ 未找到任何模型")

if __name__ == "__main__":
    main()
