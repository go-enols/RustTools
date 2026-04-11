#!/usr/bin/env python3
"""验证YOLO11下载地址"""

import requests

url = "https://github.com/ultralytics/assets/releases/download/v8.4.0/yolo11n.onnx"

print(f"测试URL: {url}")
print()

try:
    response = requests.head(url, timeout=10, allow_redirects=True)
    
    print(f"状态码: {response.status_code}")
    print(f"Content-Length: {response.headers.get('Content-Length', 'Unknown')} bytes")
    
    if response.status_code == 200:
        size_mb = int(response.headers.get('Content-Length', 0)) / (1024 * 1024)
        print(f"✓ 下载链接有效！大小: {size_mb:.2f} MB")
    else:
        print(f"✗ 下载链接无效，状态码: {response.status_code}")
        
except Exception as e:
    print(f"✗ 错误: {e}")
