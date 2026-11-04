@echo off
chcp 65001 >nul
REM =====================================================================
REM  家庭点餐系统 · 正式环境启动脚本（Windows）
REM
REM  前提：
REM    1) 已按《部署文档.md》完成数据库初始化（建库建表 + 迁移）
REM    2) 已将 .env.production.example 复制为 .env 并填好配置
REM    3) 本脚本与 bin/、dist/、uploads/、.env 在同一目录
REM
REM  用法：双击本文件，或命令行执行 start-prod.bat
REM =====================================================================

setlocal
cd /d "%~dp0"

echo.
echo ============================================
echo    家庭点餐系统 · 正式环境启动
echo ============================================
echo.

REM ---------- 检查 .env ----------
if not exist ".env" (
    echo [错误] 未找到 .env 配置文件。
    echo        请先将 .env.production.example 复制为 .env 并填写配置。
    pause
    exit /b 1
)

REM ---------- 检查二进制 ----------
if not exist "bin\family-dining-api.exe" (
    echo [错误] 未找到 bin\family-dining-api.exe。
    pause
    exit /b 1
)

REM ---------- 检查静态目录 ----------
if not exist "dist\index.html" (
    echo [警告] 未找到 dist\index.html，前端页面将无法访问。
)

echo [启动] 正在启动后端服务（Ctrl+C 停止）...
echo.
echo    浏览器访问： http://localhost:8080
echo.

REM 以当前目录为工作目录启动，确保 .env / dist / uploads 相对路径正确
"bin\family-dining-api.exe"

endlocal
pause
