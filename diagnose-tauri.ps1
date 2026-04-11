# Tauri 前端连接问题诊断

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Tauri 前端连接问题诊断工具" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 问题分析
Write-Host "【错误分析】" -ForegroundColor Yellow
Write-Host "错误: Unsafe attempt to load URL http://localhost:1420/" -ForegroundColor Red
Write-Host "原因: Vite 开发服务器未成功启动或前端页面无法访问" -ForegroundColor Gray
Write-Host ""

# 检查 1: 根目录 index.html
Write-Host "【检查 1/8】根目录 index.html..." -ForegroundColor Cyan
if (Test-Path "index.html") {
    $content = Get-Content "index.html" -Raw
    if ($content -match '<script type="module" src="/src/main.tsx"') {
        Write-Host "  ✓ index.html 存在且配置正确" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ index.html 存在但可能配置错误" -ForegroundColor Yellow
        Write-Host "  内容: $($content.Substring(0, [Math]::Min(100, $content.Length)))..." -ForegroundColor Gray
    }
} else {
    Write-Host "  ✗ index.html 不存在！这是主要问题" -ForegroundColor Red
    Write-Host "  解决方案: 已在当前目录创建 index.html" -ForegroundColor Green
}

# 检查 2: src/main.tsx
Write-Host "`n【检查 2/8】src/main.tsx..." -ForegroundColor Cyan
if (Test-Path "src/main.tsx") {
    Write-Host "  ✓ src/main.tsx 存在" -ForegroundColor Green
} else {
    Write-Host "  ✗ src/main.tsx 不存在！" -ForegroundColor Red
}

# 检查 3: src/App.tsx
Write-Host "`n【检查 3/8】src/App.tsx..." -ForegroundColor Cyan
if (Test-Path "src/App.tsx") {
    Write-Host "  ✓ src/App.tsx 存在" -ForegroundColor Green
} else {
    Write-Host "  ✗ src/App.tsx 不存在！" -ForegroundColor Red
}

# 检查 4: vite.config.ts
Write-Host "`n【检查 4/8】vite.config.ts..." -ForegroundColor Cyan
if (Test-Path "vite.config.ts") {
    $viteConfig = Get-Content "vite.config.ts" -Raw
    Write-Host "  ✓ vite.config.ts 存在" -ForegroundColor Green
    
    # 检查端口配置
    if ($viteConfig -match 'port:\s*1420') {
        Write-Host "  ✓ 端口配置为 1420" -ForegroundColor Green
    }
    
    # 检查 host 配置
    if ($viteConfig -match "host") {
        Write-Host "  ✓ 找到 host 配置" -ForegroundColor Green
    } else {
        Write-Host "  ⚠ 未找到明确的 host 配置" -ForegroundColor Yellow
    }
} else {
    Write-Host "  ✗ vite.config.ts 不存在！" -ForegroundColor Red
}

# 检查 5: tauri.conf.json
Write-Host "`n【检查 5/8】tauri.conf.json..." -ForegroundColor Cyan
if (Test-Path "src-tauri/tauri.conf.json") {
    Write-Host "  ✓ tauri.conf.json 存在" -ForegroundColor Green
    $config = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
    Write-Host "    - 产品名称: $($config.productName)" -ForegroundColor Gray
    Write-Host "    - 开发 URL: $($config.build.devUrl)" -ForegroundColor Gray
    Write-Host "    - 前端目录: $($config.build.frontendDist)" -ForegroundColor Gray
} else {
    Write-Host "  ✗ tauri.conf.json 不存在！" -ForegroundColor Red
}

# 检查 6: 端口占用
Write-Host "`n【检查 6/8】端口 1420 占用情况..." -ForegroundColor Cyan
$portCheck = netstat -ano | Select-String ":1420.*LISTENING"
if ($portCheck) {
    Write-Host "  ⚠ 端口 1420 已被占用：" -ForegroundColor Yellow
    $portCheck | ForEach-Object { 
        $parts = $_.Line -split '\s+'
        $pid = $parts[-1]
        Write-Host "    $($_)" -ForegroundColor Gray
        Write-Host "    进程 ID: $pid" -ForegroundColor Gray
    }
    Write-Host "    解决方案: 运行清理脚本关闭所有 node 进程" -ForegroundColor Cyan
} else {
    Write-Host "  ✓ 端口 1420 可用" -ForegroundColor Green
}

# 检查 7: node_modules
Write-Host "`n【检查 7/8】node_modules..." -ForegroundColor Cyan
if (Test-Path "node_modules") {
    Write-Host "  ✓ node_modules 存在" -ForegroundColor Green
    
    # 检查关键依赖
    $keyDeps = @("vite", "@vitejs/plugin-react", "@tauri-apps/api", "@tauri-apps/cli")
    $missingDeps = @()
    
    foreach ($dep in $keyDeps) {
        if (Test-Path "node_modules/$dep") {
            Write-Host "    ✓ $dep" -ForegroundColor Green
        } else {
            Write-Host "    ✗ $dep (缺失)" -ForegroundColor Red
            $missingDeps += $dep
        }
    }
    
    if ($missingDeps.Count -gt 0) {
        Write-Host "`n  建议: 运行 npm install 安装缺失的依赖" -ForegroundColor Yellow
    }
} else {
    Write-Host "  ✗ node_modules 不存在！" -ForegroundColor Red
    Write-Host "  解决方案: 运行 npm install" -ForegroundColor Green
}

# 检查 8: dist 目录（可选）
Write-Host "`n【检查 8/8】dist 目录（可选）..." -ForegroundColor Cyan
if (Test-Path "dist") {
    Write-Host "  ⚠ dist 目录存在（这是构建产物）" -ForegroundColor Yellow
    Write-Host "    对于开发模式，通常不需要 dist 目录" -ForegroundColor Gray
    Write-Host "    Vite 会直接从 src 目录提供服务" -ForegroundColor Gray
} else {
    Write-Host "  ✓ dist 目录不存在（正常）" -ForegroundColor Green
}

# 总结和建议
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "【诊断总结】" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$issues = @()

if (-not (Test-Path "index.html")) {
    $issues += "缺少根目录 index.html"
}

if ($portCheck) {
    $issues += "端口 1420 被占用"
}

if ($issues.Count -eq 0) {
    Write-Host "✓ 所有配置检查通过！" -ForegroundColor Green
    Write-Host ""
    Write-Host "请按以下步骤操作：" -ForegroundColor Yellow
    Write-Host "1. 关闭所有 node 进程: taskkill /F /IM node.exe" -ForegroundColor White
    Write-Host "2. 等待 3-5 秒" -ForegroundColor White
    Write-Host "3. 运行: npm run tauri dev" -ForegroundColor White
} else {
    Write-Host "发现以下问题：" -ForegroundColor Red
    $issues | ForEach-Object { Write-Host "  ✗ $_" -ForegroundColor Red }
    Write-Host ""
    Write-Host "正在启动自动修复..." -ForegroundColor Yellow
    
    # 自动修复
    $scriptPath = Join-Path $PSScriptRoot "start-tauri-dev.ps1"
    & $scriptPath
}
