#!/usr/bin/env python3
"""查找所有可用的YOLO模型地址"""

import requests
import json

def check_url(model_name, url, timeout=10):
    """检查URL是否可用"""
    try:
        response = requests.head(url, timeout=timeout, allow_redirects=True)
        
        if response.status_code == 200:
            size_bytes = response.headers.get('Content-Length', '0')
            size_mb = int(size_bytes) / (1024 * 1024)
            return True, size_bytes, size_mb
        else:
            return False, None, None
            
    except:
        return False, None, None

def main():
    print("查找所有可用的YOLO模型下载地址")
    print("=" * 80)
    print()
    
    # 测试不同的版本
    versions = ["v8.4.0", "v8.3.0", "v8.2.0", "v8.1.0"]
    
    # YOLO11系列
    yolo11_models = ["yolo11n", "yolo11s", "yolo11m", "yolo11l", "yolo11x"]
    
    # YOLOv8系列
    yolov8_models = ["yolov8n", "yolov8s", "yolov8m", "yolov8l", "yolov8x"]
    
    all_available = {}
    
    for version in versions:
        print(f"\n检查版本: {version}")
        print("-" * 80)
        
        base_url = f"https://github.com/ultralytics/assets/releases/download/{version}"
        
        # 检查YOLO11
        print("\nYOLO11系列:")
        for model in yolo11_models:
            url = f"{base_url}/{model}.onnx"
            available, size_bytes, size_mb = check_url(model, url)
            
            if available:
                print(f"  ✓ {model}: {size_mb:.2f} MB")
                if version not in all_available:
                    all_available[version] = {"yolo11": {}, "yolov8": {}}
                all_available[version]["yolo11"][model] = url
            else:
                print(f"  ✗ {model}")
        
        # 检查YOLOv8
        print("\nYOLOv8系列:")
        for model in yolov8_models:
            url = f"{base_url}/{model}.onnx"
            available, size_bytes, size_mb = check_url(model, url)
            
            if available:
                print(f"  ✓ {model}: {size_mb:.2f} MB")
                if version not in all_available:
                    all_available[version] = {"yolo11": {}, "yolov8": {}}
                all_available[version]["yolov8"][model] = url
            else:
                print(f"  ✗ {model}")
    
    # 总结
    print("\n" + "=" * 80)
    print("可用模型汇总")
    print("=" * 80)
    
    if all_available:
        for version, models in sorted(all_available.items()):
            print(f"\n{version}:")
            
            if models["yolo11"]:
                print("  YOLO11:")
                for model, url in models["yolo11"].items():
                    print(f"    ✓ {model}")
            
            if models["yolov8"]:
                print("  YOLOv8:")
                for model, url in models["yolov8"].items():
                    print(f"    ✓ {model}")
    else:
        print("\n✗ GitHub无法访问")
        print("\n建议:")
        print("1. 检查网络/代理设置")
        print("2. 使用ModelScope镜像: https://modelscope.cn/models/AI-ModelScope/YOLOv8n")
        print("3. 使用HuggingFace: https://huggingface.co/ultralytics/yolov8n")

if __name__ == "__main__":
    main()
