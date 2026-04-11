#!/usr/bin/env python3
"""
YOLO模型下载地址验证脚本 v3
尝试多个可能的URL格式来找到可用的下载源
"""

import requests
import time

def check_model_url(model_name, url, timeout=10):
    """检查模型URL是否可访问"""
    try:
        print(f"  {url[:70]}...", end=" ", flush=True)
        
        start_time = time.time()
        response = requests.head(url, timeout=timeout, allow_redirects=True)
        elapsed = time.time() - start_time
        
        if response.status_code == 200:
            content_length = response.headers.get('Content-Length')
            if content_length:
                size_mb = int(content_length) / (1024 * 1024)
                print(f"✓ OK ({size_mb:.2f} MB)")
                return True, size_mb
            else:
                print(f"✓ OK")
                return True, None
        else:
            print(f"✗ {response.status_code}")
            return False, None
            
    except requests.exceptions.Timeout:
        print("✗ 超时")
        return False, None
    except Exception as e:
        print(f"✗ 错误")
        return False, None

def main():
    print("=" * 80)
    print("YOLO模型下载地址验证 v3 - 多源测试")
    print("=" * 80)
    print()
    
    # 尝试不同的URL格式
    url_formats = {
        "GitHub v8.2.0": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx",
        "GitHub latest": "https://github.com/ultralytics/assets/releases/download/latest/yolov8n.onnx",
        "GitHub v8.1.0": "https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov8n.onnx",
        "HuggingFace ultralytics": "https://huggingface.co/ultralytics/yolov8n/resolve/main/yolov8n.onnx",
        "HuggingFace onnxruntime": "https://huggingface.co/onnxruntime/yolov8n/resolve/main/yolov8n.onnx",
        "ModelScope": "https://www.modelscope.cn/models/AI-ModelScope/YOLOv8n/files",
    }
    
    print("测试不同的URL格式:")
    print()
    
    working_sources = []
    
    for source_name, url in url_formats.items():
        print(f"{source_name}:")
        success, size = check_model_url("yolov8n", url)
        if success:
            working_sources.append((source_name, url))
        print()

    print("=" * 80)
    print("结果总结:")
    print("=" * 80)
    
    if working_sources:
        print(f"\n✓ 找到 {len(working_sources)} 个可用的下载源:")
        for source_name, url in working_sources:
            print(f"  • {source_name}")
            print(f"    {url}")
        
        # 建议使用第一个可用的源
        print(f"\n建议使用的下载源: {working_sources[0][0]}")
    else:
        print("\n✗ 所有数据源都不可访问")
        print("\n网络诊断:")
        print("1. 检查网络连接: ping github.com")
        print("2. 检查代理: echo %HTTP_PROXY%")
        print("3. 尝试手动下载: 访问 https://github.com/ultralytics/assets/releases")
    
    print()

if __name__ == "__main__":
    main()
