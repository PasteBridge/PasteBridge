# PasteBridge 派思桥

**派汝所思方达彼岸**

![PasteBridge Banner](public/assets/pastebridge-banner.png)

*现代化跨平台剪贴板管理器*

[官方网站](https://pastebridge.com) | [文档中心](https://docs.pastebridge.com)

---

## 特性

- 📋 **剪贴板历史** - 自动记录所有复制内容，支持文本和图片
- 🔍 **内容搜索** - 快速查找历史记录
- ⭐ **收藏功能** - 标记重要内容，永不丢失
- 🎨 **优雅界面** - 简洁美观，流畅动画
- 🔒 **本地存储** - 数据安全，完全可控
- ⚡ **全局快捷键** - 高效操作，双手不离键盘

---

## 技术栈

- **后端**: Rust
- **界面**: Slint + Skia
- **数据库**: SQLite

---

## 构建

### 环境要求

- Rust (rustup.rs)
- Visual Studio 2022 + C++ 桌面开发
- LLVM/Clang (推荐)

### 构建步骤

```powershell
# 克隆
git clone https://github.com/your-username/PasteBridge.git
cd PasteBridge

# 构建
cargo build --release

# 运行
cargo run --release
```

详细构建指南请查看 [BUILD_GUIDE.md](BUILD_GUIDE.md)

---

## 使用

1. 运行程序，主窗口显示剪贴板历史
2. 正常复制内容，自动保存到历史
3. 点击历史记录快速粘贴
4. 关闭窗口后程序最小化到托盘

---

## 目录结构

```
PasteBridge/
├── crates/
│   ├── core/          # 核心逻辑 (Rust)
│   └── desktop/ui/    # 界面文件 (Slint)
├── public/            # Web 资源
├── Cargo.toml         # 项目配置
└── BUILD_GUIDE.md     # 构建指南
```

---

## 许可证

基于 GPT-3.0 许可证。第三方库许可证详见各模块。

---

**PasteBridge - 派汝所思方达彼岸**
