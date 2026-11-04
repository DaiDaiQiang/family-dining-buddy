@echo off
chcp 65001 >nul
REM =====================================================================
REM  家庭点餐系统 · 一键启动（Windows）
REM
REM  前提：
REM    1) 已安装 MySQL 8.0 并导入 database/01_schema.sql、02_seed_data.sql
REM    2) 已安装 Rust 工具链（cargo）
REM    3) 已安装 Node.js 18+
REM
REM  用法：双击本文件，或在命令行执行 start.bat
REM =====================================================================

setlocal
cd /d "%~dp0"

echo.
echo ============================================
echo    家庭点餐系统 · 启动中
echo ============================================
echo.

REM ---------- 1. 前端 ----------
echo [1/3] 检查前端依赖...
if not exist "frontend\node_modules" (
    echo        首次运行，正在安装前端依赖（可能较慢）...
    pushd frontend
    call npm install
    popd
    if errorlevel 1 (
        echo [错误] 前端依赖安装失败，请手动执行：cd frontend ^&^& npm install
        pause
        exit /b 1
    )
)

echo [2/3] 构建前端...
pushd frontend
call npm run build
popd
if errorlevel 1 (
    echo [错误] 前端构建失败
    pause
    exit /b 1
)

REM ---------- 2. 后端 ----------
echo [3/3] 编译并启动后端（首次编译较慢）...
echo.
echo   后端启动后，浏览器访问： http://localhost:8080
echo   （若数据库未连通，前端会自动进入离线演示模式）
echo   按 Ctrl+C 停止服务
echo.
pushd backend

REM 若不存在 .env，生成默认配置（数据库账号密码请在 .env 中修改）
if not exist ".env" (
    echo   [提示] 正在生成 backend\.env 默认配置
    > .env echo # 家庭点餐系统后端配置
    >>.env echo DATABASE_URL=mysql://root:root@169.254.104.35:3306/family_dining
    >>.env echo HOST=0.0.0.0
    >>.env echo PORT=8080
    >>.env echo CORS_ORIGIN=*
    >>.env echo STATIC_DIR=../frontend/dist
    >>.env echo RUST_LOG=family_dining_api=info,actix_web=info
    >>.env echo # 快递100（后期接入物流轨迹查询；两项都填写才启用，留空不影响取件码记录）
    >>.env echo KUAIDI100_KEY=
    >>.env echo KUAIDI100_CUSTOMER=
    echo   [提示] 如数据库账号密码不同，请编辑 backend\.env 后重新运行
)

call cargo run --release
popd

endlocal
pause
