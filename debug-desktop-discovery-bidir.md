# 调试会话：桌面端单向发现 + Android 端不感知下线

**Session ID:** `desktop-discovery-bidir`
**Date:** 2026-06-11
**Status:** [OPEN]

---

## 用户报告

1. **桌面端 share panel 不显示安卓端设备** (但安卓端能看到桌面端,反之不通)
2. **关闭桌面端时,安卓端不会实时移除桌面端** (设备列表里的 desktop 条目还在)

## 假设

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | **桌面 .exe 跑的是旧二进制** (新代码已 `cargo check` 过但 .exe 被旧进程占着,relink 一直 Access denied,用户启动的可能还是旧版) | 看用户当前跑的是哪个 PID/启动时间;能不能 `cargo build` 成功并启动新版 |
| H2 | 桌面 Rust 端 `push_discovered_to_slint` 没有被调用 (browse 回调收到 android 但没推 UI) | 给 main.rs 的 browse 回调加 `[mdns-discover]` tag 详细 log + 给 `push_discovered_to_slint` 入口加 log |
| H3 | 桌面端没有给 Slint 设 `discovered-devices` (Rust 端逻辑跑到了但 set_property 没被调用) | 同 H2,看 `set_discovered_devices` 调用计数 |
| H4 | 桌面 share panel UI 没有读 `root.discovered-devices` 状态 (Slint 数据绑定断裂) | uiautomator dump / Slint inspect;或用临时写死数组测试 |
| H5 | Android `onServiceLost` 回调里只 Log.d 没有更新 `peers.value` (这就是 Issue 2 的根因) | 静态看代码 + 加 onServiceLost 内部 log |

## 证据收集 (待填)

## 根因

(待证据)

## 修复

(待证据)
