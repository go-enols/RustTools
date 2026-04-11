#!/usr/bin/env python3
"""
检查所有可用的YOLO模型URL
"""
import requests
import time
import concurrent.futures

# YOLO11 URLs (最新版本)
YOLO11_URLS = {
    "yolo11n": "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n.onnx",
    "yolo11s": "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11s.onnx",
    "yolo11m": "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11m.onnx",
    "yolo11l": "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11l.onnx",
    "yolo11x": "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11x.onnx",
}

# YOLOv8 URLs (旧版本)
YOLOV8_URLS = {
    "yolov8n": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx",
    "yolov8s": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8s.onnx",
    "yolov8m": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8m.onnx",
    "yolov8l": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8l.onnx",
    "yolov8x": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8x.onnx",
}

# YOLOv8 segmentation URLs
YOLOV8_SEG_URLS = {
    "yolov8n-seg": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n-seg.onnx",
    "yolov8s-seg": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8s-seg.onnx",
    "yolov8m-seg": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8m-seg.onnx",
    "yolov8l-seg": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8l-seg.onnx",
    "yolov8x-seg": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8x-seg.onnx",
}

# YOLOv8 pose URLs
YOLOV8_POSE_URLS = {
    "yolov8n-pose": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n-pose.onnx",
    "yolov8s-pose": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8s-pose.onnx",
    "yolov8m-pose": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8m-pose.onnx",
    "yolov8l-pose": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8l-pose.onnx",
    "yolov8x-pose": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8x-pose.onnx",
}

# YOLOv8 classify URLs
YOLOV8_CLS_URLS = {
    "yolov8n-cls": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n-cls.onnx",
    "yolov8s-cls": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8s-cls.onnx",
    "yolov8m-cls": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8m-cls.onnx",
    "yolov8l-cls": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8l-cls.onnx",
    "yolov8x-cls": "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8x-cls.onnx",
}

# 合并所有URL
ALL_URLS = {}
ALL_URLS.update(YOLO11_URLS)
ALL_URLS.update(YOLOV8_URLS)
ALL_URLS.update(YOLOV8_SEG_URLS)
ALL_URLS.update(YOLOV8_POSE_URLS)
ALL_URLS.update(YOLOV8_CLS_URLS)

def check_url(name_url):
    """检查单个URL是否有效"""
    name, url = name_url
    try:
        response = requests.head(url, timeout=10, allow_redirects=True)
        size_mb = float(response.headers.get('content-length', 0)) / (1024 * 1024)
        status = "✓ OK" if response.status_code == 200 else f"✗ {response.status_code}"
        print(f"{status:12} | {name:20} | {size_mb:8.2f} MB | {url}")
        return (name, url, response.status_code, size_mb)
    except Exception as e:
        print(f"{'✗ ERROR':12} | {name:20} | {'':8} | {url} - {str(e)}")
        return (name, url, 0, 0)

def main():
    print("=" * 120)
    print("检查所有YOLO模型URL有效性")
    print("=" * 120)
    print(f"{'状态':12} | {'模型名称':20} | {'大小':8} | URL")
    print("-" * 120)
    
    working = []
    not_working = []
    
    # 使用线程池并发检查
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
        results = list(executor.map(check_url, ALL_URLS.items()))
    
    print("\n" + "=" * 120)
    print("总结:")
    for name, url, status, size in results:
        if status == 200:
            working.append((name, url, size))
        else:
            not_working.append((name, url, status))
    
    print(f"可用模型: {len(working)}/{len(ALL_URLS)}")
    if working:
        print("\n可用的模型:")
        for name, url, size in working:
            print(f"  - {name}: {size:.2f} MB")
    
    if not_working:
        print(f"\n不可用的模型: {len(not_working)}")
        for name, url, status in not_working:
            print(f"  - {name}: HTTP {status}")
    
    print("=" * 120)
    
    # 保存可用的URL到文件
    if working:
        with open("available_models.txt", "w", encoding="utf-8") as f:
            f.write("# 可用的YOLO模型URL\n")
            f.write("# 格式: 模型名称, URL, 大小(MB)\n\n")
            for name, url, size in working:
                f.write(f"{name}|{url}|{size:.2f}\n")
        print("\n可用的模型列表已保存到 available_models.txt")

if __name__ == "__main__":
    main()
