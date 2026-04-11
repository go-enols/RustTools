#!/usr/bin/env python3
"""
YOLO模型下载地址搜索工具
尝试找到正确的模型下载链接
"""

import requests
import re

def search_github_releases():
    """搜索GitHub上的releases"""
    print("=" * 70)
    print("搜索GitHub上的YOLO模型")
    print("=" * 70)
    
    # Ultralytics assets仓库
    repo = "ultralytics/assets"
    api_url = f"https://api.github.com/repos/{repo}/releases"
    
    print(f"\n查询: {api_url}\n")
    
    try:
        response = requests.get(api_url, timeout=10)
        
        if response.status_code == 200:
            releases = response.json()
            print(f"✓ 找到 {len(releases)} 个releases\n")
            
            # 查找包含yolov8的assets
            yolo_assets = []
            for release in releases[:5]:  # 只检查前5个release
                print(f"Release: {release['tag_name']} - {release['name']}")
                
                for asset in release.get('assets', []):
                    name = asset['name']
                    if 'yolo' in name.lower() and '.onnx' in name.lower():
                        print(f"  ✓ 找到: {name}")
                        print(f"    URL: {asset['browser_download_url']}")
                        yolo_assets.append(asset)
                
                print()
            
            return yolo_assets
            
        else:
            print(f"✗ API请求失败: {response.status_code}")
            return []
            
    except Exception as e:
        print(f"✗ 错误: {e}")
        return []

def try_direct_urls():
    """尝试直接访问可能的URL"""
    print("=" * 70)
    print("尝试直接访问可能的URL")
    print("=" * 70)
    
    # 可能的URL格式
    urls_to_try = [
        # GitHub ultralytics/ultralytics
        ("GitHub ultralytics v8.2.0", "https://github.com/ultralytics/ultralytics/releases/download/v8.2.0/yolov8n.onnx"),
        ("GitHub ultralytics latest", "https://github.com/ultralytics/ultralytics/releases/download/latest/yolov8n.onnx"),
        
        # GitHub ultralytics/assets  
        ("GitHub assets v8.2.0", "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx"),
        ("GitHub assets v8.1.0", "https://github.com/ultralytics/assets/releases/download/v8.1.0/yolov8n.onnx"),
        
        # HuggingFace (不同版本)
        ("HuggingFace ultralytics main", "https://huggingface.co/ultralytics/yolov8n/resolve/main/yolov8n.onnx"),
        ("HuggingFace ultralytics main2", "https://huggingface.co/ultralytics/yolov8n/resolve/main/yolov8n-converted.onnx"),
        
        # ModelScope (不同的URL格式)
        ("ModelScope files API", "https://www.modelscope.cn/api/v1/models/AI-ModelScope/YOLOv8n/repo?Revision=master&FilePath=yolov8n.onnx"),
        
        # 官方Ultralytics Python下载（可能不是直接链接）
        ("Ultralytics docs", "https://docs.ultralytics.com/models/yolov8"),
    ]
    
    print()
    working_urls = []
    
    for name, url in urls_to_try:
        try:
            print(f"尝试: {name}...")
            response = requests.head(url, timeout=5, allow_redirects=True)
            
            if response.status_code == 200:
                size = response.headers.get('Content-Length', 'Unknown')
                print(f"  ✓ 可用 (大小: {size} bytes)")
                print(f"    {url}")
                working_urls.append((name, url, size))
            elif response.status_code in [301, 302]:
                # 重定向，检查重定向URL
                redirect_url = response.headers.get('Location', '')
                print(f"  → 重定向到: {redirect_url[:80]}")
            else:
                print(f"  ✗ 状态码: {response.status_code}")
                
        except Exception as e:
            print(f"  ✗ 错误: {e}")
        
        print()
    
    return working_urls

def search_cdn_urls():
    """搜索CDN和其他可能的来源"""
    print("=" * 70)
    print("搜索CDN和其他来源")
    print("=" * 70)
    
    urls = [
        # PyTorch Hub格式
        ("PyTorch Hub", "https://download.pytorch.org/models/yolov8n.pt"),
        
        # 其他可能的CDN
        ("Raw GitHub", "https://raw.githubusercontent.com/ultralytics/ultralytics/main/weights/yolov8n.onnx"),
        ("GitHub releases alt", "https://github.com/ultralytics/ultralytics/releases/download/8.2.0/yolov8n.onnx"),
        
        # 不同的HuggingFace仓库
        ("HF ml-community", "https://huggingface.co/ml-community/Ultralytics-YOLOv8/tree/main"),
        
        # 第三方ONNX模型库
        ("ONNX Model Zoo", "https://github.com/onnx/models/tree/main/vision/object_detection_segmentation/yolov8"),
    ]
    
    print()
    for name, url in urls:
        try:
            print(f"检查: {name}...")
            response = requests.head(url, timeout=5, allow_redirects=True)
            
            if response.status_code == 200:
                size = response.headers.get('Content-Length', 'Unknown')
                print(f"  ✓ 可用 (大小: {size} bytes)")
                print(f"    {url}")
            else:
                print(f"  ✗ 状态码: {response.status_code}")
                
        except Exception as e:
            print(f"  ✗ 错误: {e}")
        
        print()

def main():
    print("\n" + "=" * 70)
    print(" YOLO模型下载地址搜索工具")
    print("=" * 70)
    print()
    
    # 方法1: 搜索GitHub releases
    assets = search_github_releases()
    
    # 方法2: 尝试直接URL
    working = try_direct_urls()
    
    # 方法3: 搜索CDN
    search_cdn_urls()
    
    # 总结
    print("=" * 70)
    print("搜索结果总结")
    print("=" * 70)
    
    if working:
        print(f"\n✓ 找到 {len(working)} 个可用的下载链接:\n")
        for name, url, size in working:
            print(f"{name}:")
            print(f"  {url}")
            print()
    else:
        print("\n✗ 未找到可用的下载链接")
        print("\n建议:")
        print("1. 使用Python + ultralytics包下载:")
        print("   pip install ultralytics")
        print("   python -c 'from ultralytics import YOLO; m=YOLO(\"yolov8n.pt\"); m.export(format=\"onnx\")'")
        print()
        print("2. 从官网手动下载:")
        print("   https://docs.ultralytics.com/models/yolov8")
        print()
        print("3. 从GitHub手动查找:")
        print("   https://github.com/ultralytics/assets/releases")
        print("   https://github.com/ultralytics/ultralytics/releases")

if __name__ == "__main__":
    main()
