# PasteBridge 构建指南 (Windows)

## 前置条件

### 1. Rust 工具链

安装 Rust: https://rustup.rs/

确保安装 MSVC 工具链（默认即此）：
```
rustup default stable-x86_64-pc-windows-msvc
```

### 2. Visual Studio 2022

需要安装 **"使用 C++ 的桌面开发"** 工作负载（Desktop development with C++）。

- 下载: https://visualstudio.microsoft.com/zh-hans/vs/
- 在 Visual Studio Installer 中选择"使用 C++ 的桌面开发"
- 确保包含 **MSVC v143 - VS 2022 C++ x64/x86 生成工具**
- Windows 10/11 SDK

### 3. LLVM/Clang（Skia 编译需要）

Skia 构建需要 LLVM + Clang-cl。

**推荐方法** — 从 LLVM 官网下载:
https://github.com/llvm/llvm-project/releases/tag/llvmorg-19.1.7

下载 `LLVM-19.1.7-win64.exe`，安装并勾选 **"Add LLVM to the system PATH"**。

验证安装：
```
clang-cl --version
```

或设置环境变量：
```
$env:LLVM_HOME = "C:\Program Files\LLVM"
```

> 也可以使用 Scoop: `scoop install llvm`

---

## 快速开始

```powershell
# 克隆仓库
git clone <仓库地址>
cd PasteBridge

# 构建（调试模式）
cargo build

# 运行
cargo run
```

构建脚本会自动检测：
- Visual Studio 安装路径（MSVC 库目录）
- Windows Kits（UCRT 库目录）
- 链接必要的 C++ 标准库符号

---

## 构建说明

### 渲染器

本项目使用 **Skia** 渲染器（在 `Cargo.toml` 中配置）：

```toml
slint = { ..., features = ["std", "backend-winit", "renderer-skia", ...] }
```

Skia 是 GPU 加速渲染器，支持 `transform-rotation` 等硬件加速动画。

### 构建脚本 [build.rs](file:///d:/Download/PasteBridge/build.rs)

构建脚本自动完成以下工作：

1. **编译 Slint UI** — `slint_build::compile("ui/app.slint")`
2. **自动检测 MSVC 路径** — 扫描常见的 VS 安装目录，找到最高版本的 MSVC 库目录
3. **自动检测 Windows Kits 路径** — 找到 UCRT 库目录
4. **链接 C++ 标准库符号** — 编译 `msvc_stubs.cpp` 并通过 `/WHOLEARCHIVE` 强制链接

### MSVC STL 符号补丁 [msvc_stubs.cpp](file:///d:/Download/PasteBridge/msvc_stubs.cpp)

**问题背景：**

MSVC 14.40（VS 2022 17.14+）引入了新的 STL 内部函数 `__std_search_1` 和 `__std_find_first_of_trivial_pos_1`，用于优化 `std::search` 和 `std::find_first_of` 的字符查找。

预编译的 Skia 二进制由 clang-cl 编译，它在某些情况下没有内联这些函数，而是生成了外部引用。但这些函数**不在任何 MSVC 库文件**中（`msvcprt.lib` / `libcpmt.lib` 均不包含），导致链接器报错：

```
skia.lib : error LNK2019: unresolved external symbol __std_search_1
```

**解决方案：**

`msvc_stubs.cpp` 手动实现了这两个函数的等效逻辑（使用 `memcmp` / `memchr`），通过 `extern "C"` 导出为 C 链接符号。构建脚本使用 `/WHOLEARCHIVE` 强制链接器包含这些符号。

---

## 故障排除

### 1. 找不到 Visual Studio

```
error: Unable to find Visual Studio installation
```

确保安装了"使用 C++ 的桌面开发"工作负载，并重启终端。

### 2. Clang/LLVM 未找到

```
error: Unable to locate LLVM installation
```

从 https://github.com/llvm/llvm-project/releases 安装 LLVM，并设置环境变量：

```powershell
$env:LLVM_HOME = "C:\Program Files\LLVM"
```

### 3. Skia 编译失败（从源码构建）

如果预编译二进制与本地环境不兼容，可以尝试从源码构建 Skia：

```powershell
$env:SKIA_BUILD_FROM_SOURCE = "1"
cargo clean
cargo build
```

这会从头编译 Skia（首次需要较长时间，约 20-30 分钟）。

### 4. LIB 环境变量为空

正常情况下构建脚本会自动检测 MSVC 路径。如果遇到链接错误，可以手动设置：

```powershell
# 查找 MSVC 实际版本号
$msvcVer = Get-ChildItem "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC" | Select-Object -Last 1
$env:LIB = "$msvcVer\lib\x64;$env:LIB"
```

---

## 验证

构建成功后，运行程序：

```powershell
cargo run
```

右上角的 `=` 按钮点击后会以 30 度平滑展开为 `X` 形状，打开设置面板。该动画依赖 Skia 渲染器的 GPU 加速支持。