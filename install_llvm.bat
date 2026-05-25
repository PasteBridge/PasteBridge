@echo off
chcp 65001 >nul
echo ========================================
echo   LLVM 安装脚本
echo ========================================
echo.

echo 正在检查网络连接...
ping -n 1 github.com >nul 2>&1
if %errorlevel% neq 0 (
    echo 警告: GitHub 连接可能有问题
)

echo.
echo 正在下载 LLVM 18.1.8...
echo 下载地址: https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.8/LLVM-18.1.8-win64.exe
echo.

powershell -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri 'https://github.com/llvm/llvm-project/releases/download/llvmorg-18.1.8/LLVM-18.1.8-win64.exe' -OutFile 'LLVM-18.1.8-win64.exe'"

if %errorlevel% neq 0 (
    echo.
    echo 下载失败，请手动下载：
    echo 1. 访问: https://github.com/llvm/llvm-project/releases/tag/llvmorg-18.1.8
    echo 2. 下载: LLVM-18.1.8-win64.exe
    echo 3. 运行安装程序
    echo.
    echo 或者使用镜像:
    echo - https://mirror.sjtu.edu.cn/llvm-project-releases/
    echo - https://mirrors.tuna.tsinghua.edu.cn/github-release/llvm/llvm-project/
    echo.
    pause
    exit /b 1
)

echo.
echo 下载完成！正在启动安装程序...
echo.
echo 安装时请勾选: "Add LLVM to system PATH"
echo.
start LLVM-18.1.8-win64.exe

echo.
echo 安装完成后，请重新打开终端并运行：
echo   cargo build
echo.
pause
