# PasteBridge Code Wiki

## 项目概述

PasteBridge 是一个跨平台剪贴板管理器应用，使用 Rust 语言开发，Slint 框架构建 UI 界面。该项目实现了系统级剪贴板监控、历史记录存储、HTTP API 服务以及现代化的用户界面。

**主要特性：**
- 系统剪贴板实时监控
- 历史记录本地 SQLite 数据库存储
- 全局快捷键（Ctrl+Alt+V / Ctrl+Alt+B）唤起/隐藏窗口
- 系统托盘图标集成
- 悬停预览工具提示
- HTTP API 服务器（端口 18792）
- 明暗主题切换
- 内存使用监控
- 窗口淡入淡出效果

---

## 项目架构

```
PasteBridge/
├── src/
│   ├── main.rs              # 应用入口点
│   ├── lib.rs               # 库导出模块
│   ├── clipboard.rs         # 跨平台剪贴板操作
│   ├── tray.rs              # 系统托盘设置
│   ├── tooltip.rs           # 工具提示窗口
│   ├── window_effects.rs    # 窗口效果（模糊、淡入淡出）
│   ├── core/
│   │   ├── mod.rs           # 核心模块导出
│   │   ├── state.rs         # 应用状态管理
│   │   ├── clipboard.rs     # 剪贴板监控
│   │   ├── database.rs      # SQLite 数据库操作
│   │   ├── device.rs        # 设备 ID 管理
│   │   ├── memory.rs        # 内存监控
│   │   └── tray.rs          # 托盘状态
│   └── api/
│       ├── mod.rs            # API 模块导出
│       ├── server.rs         # HTTP 服务器
│       └── routes.rs         # API 路由处理
├── ui/
│   ├── app.slint            # 主 UI 定义
│   └── themes.slint         # 主题定义
├── Cargo.toml               # Rust 依赖配置
└── build.rs                 # Slint 编译构建脚本
```

---

## 核心模块职责

### 1. 主入口 (src/main.rs)

**职责：** 应用启动和初始化，负责协调各个子系统。

**主要功能：**
- 设置 Slint 后端和样式（winit-skia, fluent）
- 初始化内存监控器
- 创建应用状态和数据库
- 初始化剪贴板监控线程
- 注册全局快捷键
- 设置系统托盘
- 配置窗口回调
- 启动 HTTP API 服务器
- 运行主事件循环

**关键常量：**
```rust
const WINDOW_WIDTH: f32 = 280.0;      // 窗口宽度
const WINDOW_HEIGHT: f32 = 396.0;       // 窗口高度
const HIDDEN_WINDOW_SIZE: f32 = 1.0;     // 隐藏窗口大小
```

### 2. 核心状态管理 (src/core/state.rs)

**AppState 结构体：**
```rust
pub struct AppState {
    pub db: Mutex<Database>,           // 数据库连接
    pub max_history_size: usize,        // 最大历史记录数
    pub is_window_visible: Mutex<bool>, // 窗口可见性
    pub device_id: String,              // 设备唯一标识
}
```

**核心方法：**
| 方法 | 说明 |
|------|------|
| `new(app_data_dir, max_history_size)` | 创建新的应用状态 |
| `push_clipboard(text)` | 添加文本到剪贴板历史 |
| `push_image(data, mime_type, width, height)` | 添加图片到剪贴板历史 |
| `get_history()` | 获取历史记录列表 |
| `get_item(id)` | 获取单个记录 |
| `delete_item(id)` | 删除指定记录 |
| `toggle_favorite(id)` | 切换收藏状态 |
| `clear_history()` | 清空非收藏记录 |
| `set_window_visible(visible)` | 设置窗口可见性 |

### 3. 数据库模块 (src/core/database.rs)

**Database 结构体：** 负责所有 SQLite 数据库操作。

**ClipboardItem 结构体：**
```rust
pub struct ClipboardItem {
    pub id: i64,                       // 记录 ID
    pub content_type: ContentType,      // 内容类型 (Text/Image)
    pub content_text: Option<String>,   // 文本内容
    pub content_path: Option<String>,   // 图片路径
    pub content_hash: String,           // SHA256 哈希（去重用）
    pub mime_type: Option<String>,      // MIME 类型
    pub file_size: Option<i64>,        // 文件大小
    pub width: Option<i32>,            // 图片宽度
    pub height: Option<i32>,           // 图片高度
    pub source_ip: Option<String>,     // 来源 IP
    pub created_at: i64,               // 创建时间戳
    pub is_favorite: bool,             // 是否收藏
}
```

**数据库表结构：**
```sql
CREATE TABLE clipboard_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    content_type    TEXT NOT NULL,
    content_text    TEXT,
    content_path    TEXT,
    content_hash    TEXT NOT NULL UNIQUE,
    mime_type       TEXT,
    file_size       INTEGER,
    width           INTEGER,
    height          INTEGER,
    source_ip       TEXT,
    created_at      INTEGER NOT NULL,
    is_favorite     INTEGER NOT NULL DEFAULT 0,
    is_deleted      INTEGER NOT NULL DEFAULT 0
);
```

**关键方法：**
| 方法 | 说明 |
|------|------|
| `new(db_path, images_dir)` | 打开或创建数据库 |
| `insert_text(text)` | 插入文本记录（去重） |
| `insert_image(data, mime_type, width, height)` | 插入图片记录 |
| `get_history(limit)` | 获取历史记录（限制 200 字符） |
| `get_item(id)` | 获取单条记录 |
| `delete(id)` | 软删除记录 |
| `toggle_favorite(id)` | 切换收藏状态 |
| `clear_non_favorites()` | 清空非收藏项 |
| `compute_hash(content)` | 计算 SHA256 哈希 |

### 4. 剪贴板监控 (src/core/clipboard.rs)

**职责：** 后台线程监控系统剪贴板变化。

**工作流程：**
1. 创建独立的剪贴板监控线程
2. 每 800ms 检测剪贴板内容变化
3. 使用哈希算法检测变化（避免重复存储）
4. 内容变化时调用回调函数更新 UI

**关键函数：**
```rust
pub fn start_clipboard_monitor<F>(state: Arc<AppState>, on_change: F)
where
    F: Fn() + Send + 'static,

pub fn set_clipboard_text(text: String)

fn content_hash(text: &str) -> u64
```

### 5. 设备管理 (src/core/device.rs)

**职责：** 获取和管理设备唯一标识符。

**设备 ID 获取优先级：**
1. 读取本地保存的 `device.id` 文件
2. 读取 Windows 注册表 `MachineGuid`
3. 生成基于时间戳的备用 UUID

**关键函数：**
```rust
pub fn get_device_id(app_data_dir: &PathBuf) -> String
fn get_machine_guid() -> Option<String>
fn generate_fallback_uuid() -> String
fn rand_simple() -> u32  // xorshift32 随机数生成
```

### 6. 内存监控 (src/core/memory.rs)

**职责：** 监控应用内存使用情况。

**MemoryMonitor 结构体：**
```rust
pub struct MemoryMonitor {
    start_time: Instant,              // 启动时间
    peak_memory: AtomicU64,           // 峰值内存
    current_memory: AtomicU64,        // 当前内存
    update_count: AtomicUsize,        // 更新计数
}
```

**关键方法：**
| 方法 | 说明 |
|------|------|
| `update()` | 更新内存统计 |
| `get_current_memory()` | 获取当前内存 |
| `get_peak_memory()` | 获取峰值内存 |
| `get_uptime()` | 获取运行时间 |
| `format_memory(bytes)` | 格式化内存显示 |

### 7. HTTP API 服务器 (src/api/server.rs)

**职责：** 提供 HTTP 接口供外部应用访问剪贴板数据。

**服务器配置：**
- 端口：18792
- 地址：127.0.0.1

**API 端点：**

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/clipboard/history` | 获取剪贴板历史 |
| POST | `/clipboard/copy` | 复制内容到剪贴板 |
| POST | `/clipboard/clear` | 清空历史记录 |
| GET | `/window/visible` | 获取窗口可见性 |
| POST | `/window/show` | 显示窗口 |
| POST | `/window/hide` | 隐藏窗口 |

### 8. 系统托盘 (src/tray.rs)

**职责：** 管理系统托盘图标和菜单。

**TrayHandles 结构体：**
```rust
pub struct TrayHandles {
    pub show_id: String,           // 显示/隐藏菜单项 ID
    pub quit_id: String,           // 退出菜单项 ID
    pub tray_icon: tray_icon::TrayIcon,
}
```

**托盘菜单项：**
- "Show/Hide" - 切换窗口显示
- "Quit PasteBridge" - 退出应用

### 9. 工具提示 (src/tooltip.rs)

**职责：** 创建和管理悬停预览窗口。

**窗口类型：**
- **标准工具提示** - 跟随鼠标，显示 "Copied" 等提示
- **悬停预览窗口** - 固定在列表左侧，显示剪贴板内容预览

**关键函数：**
| 函数 | 说明 |
|------|------|
| `show_tooltip_at(x, y, text)` | 在指定位置显示提示 |
| `show_hover_tooltip(text)` | 显示内容预览窗口 |
| `hide_hover_tooltip()` | 隐藏预览窗口 |
| `get_cursor_pos()` | 获取鼠标位置 |

### 10. 窗口效果 (src/window_effects.rs)

**职责：** 实现窗口视觉效果（淡入淡出、模糊背景）。

**全局状态：**
```rust
pub static APP_HWND: AtomicIsize = AtomicIsize::new(0);
pub static WINDOW_EFFECTS_READY: AtomicBool = AtomicBool::new(false);
pub static INITIAL_FADE_DONE: AtomicBool = AtomicBool::new(false);
```

**关键函数：**
| 函数 | 说明 |
|------|------|
| `apply_window_effects()` | 应用窗口效果（模糊背景、DWM 扩展） |
| `fade_in(hwnd)` | 淡入动画（15 步，每步 5ms） |
| `fade_out(hwnd)` | 淡出动画（15 步，每步 5ms） |
| `wait_for_window_effects_ready()` | 等待效果准备完成 |

---

## UI 模块 (ui/app.slint)

### 组件结构

**AppWindow 主窗口：**
- 尺寸：280x396 像素
- 无边框、可调整大小
- 始终在最前
- 支持拖拽移动

### 回调函数 (Callbacks)

| 回调 | 参数 | 说明 |
|------|------|------|
| `minimize-window` | - | 最小化窗口到托盘 |
| `hide-window` | - | 隐藏窗口 |
| `start-drag` | - | 开始拖拽窗口 |
| `copy-item` | index: int | 复制指定索引的内容 |
| `clear-history` | - | 清空历史记录 |
| `toggle-settings` | - | 切换设置面板 |
| `show-hover-tooltip` | text: string | 显示预览提示 |
| `show-hover-tooltip-index` | index: int | 根据索引显示预览 |
| `hide-hover-tooltip` | - | 隐藏预览提示 |
| `toggle-theme` | - | 切换明暗主题 |

### 属性 (Properties)

| 属性 | 类型 | 说明 |
|------|------|------|
| `clipboard-history` | `[string]` | 剪贴板历史列表 |
| `placeholder-text` | `string` | 占位文本 |
| `settings-visible` | `bool` | 设置面板可见性 |

### 界面布局

```
┌─────────────────────────────────────────┐
│  ┌─────────────────────────────────┐  ┌─┤
│  │                                 │  │ │
│  │    Clipboard History List       │  │ │
│  │    (可滚动区域)                  │  │ │
│  │                                 │  │D│
│  │  ┌─────────────────────────┐   │  │R│
│  │  │   History Item 1         │   │  │A│
│  │  └─────────────────────────┘   │  │G│
│  │  ┌─────────────────────────┐   │  │ │
│  │  │   History Item 2         │   │  │ │
│  │  └─────────────────────────┘   │  │ │
│  │                                 │  │ │
│  ├─────────────────────────────────┤  │ │
│  │         Search Box              │  │ │
│  └─────────────────────────────────┘  │ │
│                                        │─┘│
└─────────────────────────────────────────┘
```

**侧边栏按钮：**
- 🗑️ 清空历史
- ❌ 隐藏窗口
- ⚙️ 设置
- 📤 分享

---

## 依赖关系

### Cargo.toml 依赖

```toml
[dependencies]
# UI 框架
slint = { version = "1.8", features = ["std", "backend-winit", "renderer-femtovg"] }

# 系统集成
global-hotkey = "0.6"           # 全局快捷键
clipboard-win = "5.4"           # Windows 剪贴板
arboard = "3.4"                 # 跨平台剪贴板

# 数据存储
rusqlite = { version = "0.32", features = ["bundled"] }  # SQLite

# 加密
sha2 = "0.10"                   # SHA256 哈希
hex = "0.4"                      # 十六进制编码

# HTTP 服务器
tiny_http = "0.12"              # HTTP 服务器

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Windows API
windows = { version = "0.58", features = [...] }
raw-window-handle = "0.6.2"
window-vibrancy = "0.7.1"
tray-icon = "0.14"

[build-dependencies]
slint-build = "1.8"
```

### 模块依赖图

```
┌─────────────────────────────────────────────────────────┐
│                      main.rs                            │
│  - 初始化所有组件                                        │
│  - 协调各模块工作                                        │
└─────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│   core/state  │    │    api/      │    │   UI 层       │
│               │    │               │    │               │
│ - AppState    │    │ - ApiServer  │    │ - app.slint   │
│ - Database    │    │ - routes     │    │ - themes      │
│ - clipboard   │    │               │    │               │
└───────────────┘    └───────────────┘    └───────────────┘
        │                     │                     │
        ▼                     │                     ▼
┌───────────────┐             │           ┌───────────────┐
│  系统层       │             │           │   窗口效果    │
│               │             │           │               │
│ - tray.rs     │◄────────────┘           │ - tooltip    │
│ - clipboard   │  状态共享                │ - effects    │
│ - memory      │                         │ - tray       │
│ - device      │                         └───────────────┘
└───────────────┘
```

---

## 全局快捷键

| 快捷键 | 说明 | 优先级 |
|--------|------|--------|
| Ctrl+Alt+V | 显示/隐藏主窗口 | 首选 |
| Ctrl+Alt+B | 显示/隐藏主窗口 | 备用（首选被占用时）|

**快捷键注册流程：**
1. 尝试注册 Ctrl+Alt+V
2. 如果失败，回退到 Ctrl+Alt+B
3. 如果备用也失败，继续运行但无快捷键支持

---

## 数据存储

### 存储路径
```
%LOCALAPPDATA%\PasteBridge\
├── clipboard.db       # SQLite 数据库
├── images\            # 剪贴板图片目录
│   ├── xxxxxxxx.png
│   └── xxxxxxxx.jpg
└── device.id         # 设备唯一标识
```

### 数据库去重机制
- 使用 SHA256(content) 作为内容哈希
- 重复内容更新 `created_at` 时间戳
- 相同内容不会重复创建记录

---

## 运行方式

### 开发模式

```bash
# 构建项目
cargo build

# 运行项目
cargo run
```

### 发布构建

```bash
# Release 优化构建
cargo build --release

# 二进制文件位于
# target/release/paste_bridge.exe
```

### 构建配置
```toml
[profile.release]
opt-level = "z"         # 优化大小
lto = true             # 链接时优化
codegen-units = 1      # 降低编译单元
panic = "abort"        # 移除 panic 信息
strip = true           # 去除符号信息
```

### 环境变量
```bash
# 设置 Slint 后端
export SLINT_BACKEND=winit-skia

# 设置 Slint 样式
export SLINT_STYLE=fluent
```

---

## API 使用示例

### 获取剪贴板历史
```bash
curl http://127.0.0.1:18792/clipboard/history
```

### 复制内容到剪贴板
```bash
curl -X POST -d "Hello World" http://127.0.0.1:18792/clipboard/copy
```

### 显示窗口
```bash
curl -X POST http://127.0.0.1:18792/window/show
```

### 隐藏窗口
```bash
curl -X POST http://127.0.0.1:18792/window/hide
```

---

## 注意事项

1. **内存优化**：历史记录中的文本内容被限制为前 200 个字符，全文按需加载
2. **线程安全**：核心状态通过 `Mutex` 保护，剪贴板监控在独立线程运行
3. **Windows 专用**：部分模块使用了 Windows API，仅支持 Windows 平台
4. **API 本地访问**：HTTP API 仅监听 127.0.0.1，不支持远程访问
5. **设备标识**：设备 ID 用于标识不同设备，方便未来跨设备同步

---

## 扩展计划

根据 README，项目计划支持：
- Android App
- iOS App
- Windows App（当前为无边框窗口）
- 跨设备同步功能
