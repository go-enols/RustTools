#!/usr/bin/env python3
"""
尝试使用ultralytics Python包下载模型并获取URL
"""
import subprocess
import sys
import json

def try_ultralytics_install():
    """尝试安装ultralytics并获取模型URL"""
    print("尝试使用ultralytics包获取模型信息...")
    
    # 尝试导入ultralytics
    try:
        from ultralytics import YOLO
        print("✓ ultralytics已安装")
        
        # 下载模型并获取路径
        model = YOLO('yolo11n.pt')
        print(f"✓ 模型已下载: {model.model_path}")
        
        # 尝试导出为ONNX
        onnx_path = model.export(format='onnx')
        print(f"✓ ONNX导出成功: {onnx_path}")
        
        return True
    except ImportError:
        print("✗ ultralytics未安装")
        return False
    except Exception as e:
        print(f"✗ 错误: {e}")
        return False

def try_direct_huggingface():
    """尝试从HuggingFace下载"""
    print("\n检查HuggingFace上的模型...")
    
    # HuggingFace上的Ultralytics模型
    hf_models = [
        "bighoon4/yolo11n",
        "bighoon4/yolo11s", 
        "bighoon4/yolo11m",
        "nateram/YOLO11n",
    ]
    
    for model_id in hf_models:
        url = f"https://huggingface.co/{model_id}/resolve/main/model.onnx"
        print(f"检查 {url}...", end=" ")
        try:
            import requests
            response = requests.head(url, timeout=10, allow_redirects=True)
            if response.status_code == 200:
                size = int(response.headers.get('content-length', 0)) / (1024 * 1024)
                print(f"✓ {size:.2f} MB")
            else:
                print(f"✗ HTTP {response.status_code}")
        except Exception as e:
            print(f"✗ {e}")

def check_github_api():
    """使用GitHub API搜索ONNX文件"""
    print("\n使用GitHub API搜索YOLO ONNX文件...")
    
    import requests
    
    # 搜索包含YOLO和ONNX的releases
    search_urls = [
        "https://api.github.com/repos/ultralytics/assets/releases/latest",
    ]
    
    for url in search_urls:
        try:
            response = requests.get(url, timeout=10)
            if response.status_code == 200:
                data = response.json()
                print(f"\n最新Release: {data.get('tag_name', 'unknown')}")
                print(f"名称: {data.get('name', 'unknown')}")
                print(f"发布于: {data.get('published_at', 'unknown')[:10]}")
                
                assets = data.get('assets', [])
                onnx_assets = [a for a in assets if a['name'].endswith('.onnx')]
                
                print(f"\nONNX模型 ({len(onnx_assets)}个):")
                for asset in onnx_assets:
                    size_mb = asset['size'] / (1024 * 1024)
                    print(f"  - {asset['name']:40} | {size_mb:7.2f} MB")
                
                # 保存URLs
                with open('github_latest_onnx.json', 'w') as f:
                    json.dump({
                        'tag': data.get('tag_name'),
                        'assets': [{
                            'name': a['name'],
                            'url': a['browser_download_url'],
                            'size_mb': a['size'] / (1024 * 1024)
                        } for a in onnx_assets]
                    }, f, indent=2)
                print("\n✓ URLs已保存到 github_latest_onnx.json")
                return True
        except Exception as e:
            print(f"✗ 错误: {e}")
    
    return False

def main():
    print("="*80)
    print("YOLO模型URL搜索工具")
    print("="*80)
    
    # 方法1: 检查GitHub API
    check_github_api()
    
    # 方法2: 检查HuggingFace
    try_direct_huggingface()
    
    # 方法3: 尝试ultralytics
    print("\n" + "="*80)
    print("提示: 如果需要其他模型，建议:")
    print("1. 使用Python的ultralytics包下载: pip install ultralytics")
    print("2. 然后导出为ONNX: model.export(format='onnx')")
    print("="*80)

if __name__ == "__main__":
    main()
