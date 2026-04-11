#!/usr/bin/env python3
"""
检查多个YOLO模型下载源
"""
import requests
import json

# 定义要测试的模型
models_to_test = [
    # YOLO11 - GitHub (v8.3.0)
    ("yolo11n", "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11n.onnx"),
    ("yolo11s", "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11s.onnx"),
    ("yolo11m", "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11m.onnx"),
    ("yolo11l", "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11l.onnx"),
    ("yolo11x", "https://github.com/ultralytics/assets/releases/download/v8.3.0/yolo11x.onnx"),
    
    # YOLOv8 - GitHub (v8.2.0)
    ("yolov8n", "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx"),
    ("yolov8s", "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8s.onnx"),
    ("yolov8m", "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8m.onnx"),
    ("yolov8l", "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8l.onnx"),
    ("yolov8x", "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8x.onnx"),
    
    # ModelScope URLs (中国镜像)
    ("yolo11n-modelscope", "https://www.modelscope.cn/models/AI-ModelScope/YOLOv11n/resolve/master/yolo11n.onnx"),
    ("yolo11s-modelscope", "https://www.modelscope.cn/models/AI-ModelScope/YOLOv11s/resolve/master/yolo11s.onnx"),
    ("yolov8n-modelscope", "https://www.modelscope.cn/models/AI-ModelScope/YOLOv8n/resolve/master/yolov8n.onnx"),
    ("yolov8s-modelscope", "https://www.modelscope.cn/models/AI-ModelScope/YOLOv8s/resolve/master/yolov8s.onnx"),
    ("yolov8m-modelscope", "https://www.modelscope.cn/models/AI-ModelScope/YOLOv8m/resolve/master/yolov8m.onnx"),
]

def check_url(name, url):
    """检查URL是否可访问"""
    try:
        # 使用HEAD请求，只获取头部信息
        response = requests.head(url, timeout=15, allow_redirects=True)
        content_length = response.headers.get('content-length')
        size_mb = float(content_length) / (1024 * 1024) if content_length else 0
        
        return {
            'name': name,
            'url': url,
            'status': response.status_code,
            'size_mb': size_mb,
            'ok': response.status_code == 200
        }
    except Exception as e:
        return {
            'name': name,
            'url': url,
            'status': 0,
            'size_mb': 0,
            'ok': False,
            'error': str(e)
        }

def main():
    print("="*120)
    print("YOLO模型多源检查")
    print("="*120)
    
    results = []
    for name, url in models_to_test:
        print(f"检查 {name}...", end=" ", flush=True)
        result = check_url(name, url)
        results.append(result)
        if result['ok']:
            print(f"✓ {result['size_mb']:.2f} MB")
        elif result['status'] == 0:
            print(f"✗ 错误: {result.get('error', '未知')}")
        else:
            print(f"✗ HTTP {result['status']}")
    
    print("\n" + "="*120)
    print("总结:")
    print("="*120)
    
    working = [r for r in results if r['ok']]
    not_working = [r for r in results if not r['ok']]
    
    print(f"\n可用的模型: {len(working)}/{len(results)}")
    if working:
        print("\n可用的下载源:")
        for r in working:
            print(f"  ✓ {r['name']:30} | {r['size_mb']:7.2f} MB | {r['url']}")
    
    if not_working:
        print(f"\n不可用的下载源: {len(not_working)}")
        for r in not_working:
            error_info = f" - {r.get('error', '')}" if r.get('error') else ""
            print(f"  ✗ {r['name']:30} | HTTP {r['status']}{error_info}")
    
    # 保存结果
    with open('model_urls_check.json', 'w', encoding='utf-8') as f:
        json.dump({
            'working': working,
            'not_working': not_working
        }, f, indent=2, ensure_ascii=False)
    
    print(f"\n✓ 检查结果已保存到 model_urls_check.json")
    
    # 生成Rust可用的代码
    if working:
        print("\n" + "="*120)
        print("Rust代码 - 可用的模型:")
        print("="*120)
        print('''
#[derive(Debug, Clone)]
pub struct ModelDownloadInfo {
    pub name: String,
    pub url: String,
    pub size_mb: f64,
}

pub fn get_available_model_urls() -> Vec<ModelDownloadInfo> {
    vec![
''')
        for r in working:
            print(f'        ModelDownloadInfo {{')
            print(f'            name: "{r["name"]}".to_string(),')
            print(f'            url: "{r["url"]}".to_string(),')
            print(f'            size_mb: {r["size_mb"]:.2f},')
            print(f'        }},')
        print('    ]')
        print('}')

if __name__ == "__main__":
    main()
