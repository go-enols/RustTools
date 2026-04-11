#!/usr/bin/env python3
"""
从GitHub API获取Ultralytics assets的所有可用模型
"""
import requests
import json

def get_github_release_assets(owner="ultralytics", repo="assets", tag="8.4.0"):
    """从GitHub API获取指定版本的assets列表"""
    url = f"https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}"
    
    try:
        response = requests.get(url, timeout=30)
        if response.status_code == 200:
            data = response.json()
            assets = data.get('assets', [])
            
            print(f"\n{'='*80}")
            print(f"GitHub Release: {tag}")
            print(f"{'='*80}\n")
            
            models = {}
            for asset in assets:
                name = asset['name']
                size_mb = asset['size'] / (1024 * 1024)
                browser_download_url = asset['browser_download_url']
                
                # 只关注ONNX模型
                if name.endswith('.onnx'):
                    print(f"✓ {name:40} | {size_mb:8.2f} MB")
                    models[name] = {
                        'url': browser_download_url,
                        'size_mb': size_mb
                    }
            
            print(f"\n共找到 {len(models)} 个ONNX模型")
            return models
        else:
            print(f"✗ 无法获取release信息: HTTP {response.status_code}")
            return None
    except Exception as e:
        print(f"✗ 请求失败: {e}")
        return None

def get_all_releases(owner="ultralytics", repo="assets"):
    """获取所有releases列表"""
    url = f"https://api.github.com/repos/{owner}/{repo}/releases"
    
    try:
        response = requests.get(url, timeout=30)
        if response.status_code == 200:
            releases = response.json()
            print(f"\n找到 {len(releases)} 个releases:\n")
            
            for i, release in enumerate(releases[:10]):  # 只显示前10个
                tag = release['tag_name']
                name = release['name']
                date = release['published_at'][:10]
                assets_count = len(release.get('assets', []))
                print(f"{i+1:2}. {tag:20} | {name:30} | {date} | {assets_count} assets")
            
            return releases
        else:
            print(f"✗ 无法获取releases列表: HTTP {response.status_code}")
            return None
    except Exception as e:
        print(f"✗ 请求失败: {e}")
        return None

def main():
    print("="*80)
    print("YOLO模型URL搜索工具")
    print("="*80)
    
    # 先获取所有releases
    releases = get_all_releases()
    
    if releases and len(releases) > 0:
        # 检查最新的几个版本的assets
        for release in releases[:3]:
            tag = release['tag_name']
            get_github_release_assets(tag=tag)
            print()

if __name__ == "__main__":
    main()
