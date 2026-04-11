#!/usr/bin/env python3
"""验证所有YOLO模型下载地址"""

import requests
import json

def check_url(model_name, url, timeout=10):
    """检查URL是否可用"""
    try:
        print(f"检查 {model_name:15}...", end=" ", flush=True)
        response = requests.head(url, timeout=timeout, allow_redirects=True)
        
        if response.status_code == 200:
            size_bytes = response.headers.get('Content-Length', '0')
            size_mb = int(size_bytes) / (1024 * 1024)
            print(f"✓ OK ({size_mb:.2f} MB)")
            return True, size_bytes
        else:
            print(f"✗ {response.status_code}")
            return False, None
            
    except Exception as e:
        print(f"✗ 错误")
        return False, None

def main():
    print("=" * 80)
    print("验证所有YOLO模型下载地址")
    print("=" * 80)
    print()
    
    # 所有YOLO模型列表
    models = {
        "YOLO11系列": {
            "base_url": "https://github.com/ultralytics/assets/releases/download/v8.4.0",
            "models": [
                ("yolo11n", "yolo11n.onnx"),
                ("yolo11s", "yolo11s.onnx"),
                ("yolo11m", "yolo11m.onnx"),
                ("yolo11l", "yolo11l.onnx"),
                ("yolo11x", "yolo11x.onnx"),
            ]
        },
        "YOLOv8系列": {
            "base_url": "https://github.com/ultralytics/assets/releases/download/v8.2.0",
            "models": [
                ("yolov8n", "yolov8n.onnx"),
                ("yolov8s", "yolov8s.onnx"),
                ("yolov8m", "yolov8m.onnx"),
                ("yolov8l", "yolov8l.onnx"),
                ("yolov8x", "yolov8x.onnx"),
            ]
        },
        "YOLOv8变体": {
            "base_url": "https://github.com/ultralytics/assets/releases/download/v8.2.0",
            "models": [
                ("yolov8n-pose", "yolov8n-pose.onnx"),
                ("yolov8s-pose", "yolov8s-pose.onnx"),
                ("yolov8m-pose", "yolov8m-pose.onnx"),
                ("yolov8l-pose", "yolov8l-pose.onnx"),
                ("yolov8x-pose", "yolov8x-pose.onnx"),
                ("yolov8n-seg", "yolov8n-seg.onnx"),
                ("yolov8s-seg", "yolov8s-seg.onnx"),
                ("yolov8m-seg", "yolov8m-seg.onnx"),
                ("yolov8l-seg", "yolov8l-seg.onnx"),
                ("yolov8x-seg", "yolov8x-seg.onnx"),
                ("yolov8n-cls", "yolov8n-cls.onnx"),
                ("yolov8s-cls", "yolov8s-cls.onnx"),
                ("yolov8m-cls", "yolov8m-cls.onnx"),
                ("yolov8l-cls", "yolov8l-cls.onnx"),
                ("yolov8x-cls", "yolov8x-cls.onnx"),
            ]
        }
    }
    
    all_results = {}
    
    for category, config in models.items():
        print(f"\n{category}:")
        print("-" * 80)
        
        base_url = config["base_url"]
        results = {}
        
        for model_name, filename in config["models"]:
            url = f"{base_url}/{filename}"
            success, size = check_url(model_name, url)
            results[model_name] = {
                "url": url,
                "available": success,
                "size": size
            }
        
        all_results[category] = results
    
    # 统计
    print("\n" + "=" * 80)
    print("验证结果统计")
    print("=" * 80)
    
    for category, results in all_results.items():
        available = sum(1 for r in results.values() if r["available"])
        total = len(results)
        print(f"\n{category}: {available}/{total} 可用")
        
        for name, info in results.items():
            if info["available"]:
                size_mb = int(info["size"]) / (1024 * 1024)
                print(f"  ✓ {name}: {size_mb:.2f} MB")
            else:
                print(f"  ✗ {name}")
    
    # 生成Rust代码
    print("\n" + "=" * 80)
    print("Rust代码（复制到trainer.rs）")
    print("=" * 80)
    print()
    
    rust_code = '''    // YOLO预训练模型映射
    let model_urls = HashMap::from([
'''
    
    for category, results in all_results.items():
        for name, info in results.items():
            if info["available"]:
                rust_code += f'        ("{name}", "{info["url"]}"),\n'
    
    rust_code += '    ]);'
    
    print(rust_code)
    
    # 保存结果到JSON
    output_file = "model_urls_verified.json"
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(all_results, f, indent=2, ensure_ascii=False)
    
    print(f"\n结果已保存到: {output_file}")

if __name__ == "__main__":
    main()
