# PasteBridge 项目进度检查 - 2026-06-11

## 项目概览

**PasteBridge** 是一个跨平台剪贴板同步工具，使用 Rust + Slint 开发。

## 技术架构

- **后端**: Rust (crates/core)
- **桌面端**: Slint + Skia (crates/desktop)
- **移动端**: Kotlin (KMP) + UniFFI
- **网络**: mDNS 发现 + HTTP/JSON 同步

## 当前进度状态

### 已完成模块

| 模块 | 状态 | 验证 |
|------|------|------|
| Desktop 核心 (Rust) | 已完成 | SQLite 持久化、mDNS、HTTP API |
| Slint UI 界面 | 已完成 | 剪贴板历史、收藏夹、动画 |
| UniFFI C-ABI 绑定 | 已完成 | 已生成 Kotlin 绑定 |
| Android Discovery | 已完成 | 基于 Rust 的 mDNS 发现 |
| PC ↔ Phone 双向 HTTP | 已完成 | curl 测试通过 |
| Unicast TCP 检测 | 已完成 | 7 个端口、2 条线程 |
| sync_with_peer FFI 函数 | 已完成 | #[uniffi::export] 已添加 |

### 待解决问题

| 问题 | 影响 |
|------|------|
| **mDNS 双向不通** | Windows 多网卡导致 mDNS 无法跨网发现 |
| **Android HTTP Server 未启动** | ApiServer 无回调，18792 端口为空 |
| **Android sync 未实现** | sync_with_device 未通过 UniFFI 导出 |
| **Android 历史数据为 mock** | 同步成功也无实际数据交换 |
| **iOS 端完全空白** | 仅有桩代码 |

## 关键文件路径

| 文件 | 说明 |
|------|------|
| crates/core/src/sync_device.rs | sync 核心逻辑 + FFI-safe 入口 |
| crates/core/src/discovery.rs | mDNS register/browse + unicast fallback |
| crates/core/src/api/server.rs | HTTP ApiServer (callback mode) |
| crates/core/src/lib.rs | UniFFI scaffold |
| crates/desktop/src/callbacks.rs | 桌面端 sync 按钮回调 |

## 编译状态

```
cargo build -p paste_bridge_core  # 已完成
```

## 下一步计划

1. 重新生成 Kotlin 绑定 (uniffi-bindgen generate)
2. Android 端实现 ClipboardApiCallback + 启动 ApiServer
3. Android 端 "Sync" 按钮调用 sync_with_peer
4. 验证 Desktop ↔ Android 双向发现 + sync

## 调试命令备忘

```sh
# 构建 core 库
cargo build -p paste_bridge_core

# 生成 UniFFI Kotlin 绑定
uniffi-bindgen generate --library target/debug/paste_bridge_core.dll --language kotlin --out-dir crates/mobile/shared/src/androidMain/kotlin/uniffi/paste_bridge_core/generated

# 启动 phone HTTP server
adb -s <device> shell "nohup sh /data/local/tmp/pb_dummy_server.sh 18792 > /dev/null 2>&1 &"

# Rust TCP 探测
target/debug/examples/tcp_probe.exe 10.239.38.89:18792

# 端到端 sync
target/debug/examples/phone_sync_smoke.exe 10.239.38.89:18792
```
