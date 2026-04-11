#!/usr/bin/env python3
"""
ModelScope YOLO模型下载脚本
用于从ModelScope下载YOLOv8 ONNX模型
"""

import os
import sys

def get_models():
    """获取可用的模型列表"""
    return {
        "yolov8n": "AI-ModelScope/YOLOv8n",
        "yolov8s": "AI-ModelScope/YOLOv8s",
        "yolov8m": "AI-ModelScope/YOLOv8m",
        "yolov8l": "AI-ModelScope/YOLOv8l",
        "yolov8x": "AI-ModelScope/YOLOv8x",
    }

def download_with_cli(model_name, repo_id):
    """使用modelscope CLI下载"""
    import subprocess
    
    print(f"正在下载 {model_name}...")
    print(f"模型ID: {repo_id}")
    
    try:
        # 尝试使用modelscope CLI
        cmd = [
            sys.executable, "-m", "modelscope.cli.cli",
            "model", "download",
            "--model", repo_id,
            "--file", "yolov8n.onnx",  # 假设文件名
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode == 0:
            print(f"✓ {model_name} 下载成功")
            print(result.stdout)
            return True
        else:
            print(f"✗ {model_name} 下载失败")
            print(result.stderr)
            return False
            
    except Exception as e:
        print(f"✗ 下载出错: {e}")
        return False

def download_with_api(token, model_name, repo_id):
    """使用API下载"""
    try:
        from modelscope.hub.api import HubApi
        
        print(f"正在下载 {model_name}...")
        api = HubApi()
        api.login(token)
        
        # 获取模型文件
        model_dir = api.model_download(repo_id)
        print(f"✓ {model_name} 下载成功，保存到: {model_dir}")
        return True
        
    except Exception as e:
        print(f"✗ 下载出错: {e}")
        return False

def main():
    print("=" * 60)
    print("ModelScope YOLO模型下载工具")
    print("=" * 60)
    print()
    
    models = get_models()
    
    print("可用的模型:")
    for name, repo_id in models.items():
        print(f"  • {name}: {repo_id}")
    print()
    
    # 让用户选择模型
    print("使用方法:")
    print("1. 方法一（推荐）：手动下载")
    print("   - 访问 https://www.modelscope.cn/models/AI-ModelScope/YOLOv8n")
    print("   - 点击下载按钮")
    print("   - 将文件保存到 ~/.cache/rust-tools/models/")
    print()
    print("2. 方法二：使用Token下载")
    print("   - 获取Token: https://modelscope.cn/my/token")
    print("   - 设置环境变量: export MODELSCOPE_TOKEN=your_token")
    print("   - 运行: python -m modelscope.hub.api model download AI-ModelScope/YOLOv8n")
    print()
    
    # 检查是否设置了token
    token = os.environ.get("MODELSCOPE_TOKEN")
    if token:
        print("✓ 检测到ModelScope Token，将自动下载")
        print()
        
        for model_name, repo_id in models.items():
            download_with_api(token, model_name, repo_id)
            print()
    else:
        print("✗ 未设置ModelScope Token")
        print()
        print("请选择:")
        print("1. 手动下载（推荐）")
        print("2. 配置Token后自动下载")
        
        choice = input("请选择 (1/2): ").strip()
        
        if choice == "2":
            token = input("请输入ModelScope Token: ").strip()
            if token:
                os.environ["MODELSCOPE_TOKEN"] = token
                for model_name, repo_id in models.items():
                    download_with_api(token, model_name, repo_id)
                    print()
            else:
                print("未输入Token，退出")
        else:
            print()
            print("手动下载说明:")
            print("1. 访问以下URL下载模型:")
            for name, repo_id in models.items():
                print(f"   - {name}: https://www.modelscope.cn/models/{repo_id}")
            print()
            print("2. 将下载的模型文件保存到:")
            cache_dir = os.path.expanduser("~/.cache/rust-tools/models")
            print(f"   {cache_dir}")
            print()
            print("3. 文件命名:")
            for name in models.keys():
                print(f"   - {name}.onnx")

if __name__ == "__main__":
    main()
