# 调试会话：跨设备 mDNS LAN 发现

**Session ID:** `mdns-lan-discovery`
**Date:** 2026-06-11
**Status:** [CLOSED — RESOLVED]

---

## 用户报告

Android 真机 (85e3eaa9, Android 16) + Windows PC 已连同一路由器。需要验证 mDNS 双向发现。

## 假设（开始收集证据前，禁止改业务代码）

| ID | 假设 | 验证方式 |
|----|------|---------|
| H1 | Android 设备实际**未**在 Wi-Fi 上（可能仍是蜂窝 / Wi-Fi 切换未生效）| `adb shell ip route`, `dumpsys wifi`, Wi-Fi 是否 enabled |
| H2 | Android 在 Wi-Fi 上但路由器**屏蔽 mDNS 组播**（AP 隔离 / guest 网络 / 5GHz 客户端隔离）| 路由器型号未知；用 `ping 224.0.0.1` 测组播是否在本机发出 |
| H3 | Android NSD `onServiceRegistered` 回调未触发（与蜂窝时观察到的现象一致）| `adb logcat` 抓 `PBDiscovery` + `serviceDiscovery` |
| H4 | Windows 桌面 `mdns-sd` 注册成功但 Bonjour service 缺失，**只能发不能收** | `cargo run` 桌面 + 看 `[mdns]` 日志；用 `Get-Service Bonjour` 验证 |
| H5 | 服务类型 / 端口 / deviceId 任一不匹配导致**单边能注册、跨端互不相认** | 比对 Rust `SERVICE_TYPE = "_pastebridge._tcp.local."` vs Android `SERVICE_TYPE = "_pastebridge._tcp."` |

## 调试策略

1. 静态核对 Rust/Android 服务类型常量（消除 H5）
2. 收集 Android 端网络环境证据（消除 H1/H2）
3. 收集 Android 端 mDNS 注册证据（消除 H3）
4. 收集 Windows 端 mDNS 注册 + 浏览证据（消除 H4）
5. 跨端联调，看哪一边是发现链路的断点

## 证据收集（每条假设一行）

| ID | 假设 | 证据 | 结论 |
|----|------|------|------|
| H1 | Android 未在 Wi-Fi | `adb shell ip route` 显示 `192.168.5.0/24 dev wlan0 ... src 192.168.5.130` | **FALSIFIED** ✅ |
| H2 | 路由器屏蔽 mDNS 组播 | Android NSD 在 16:20:21 成功接收 `_kdeconnect._udp` 响应 (`ip4: [192.168.5.130]`) | **FALSIFIED** ✅ |
| H3 | Android NSD `onServiceRegistered` 回调不触发 | 16:13:32 在 wlan0 上 `onServiceRegistered: PasteBridge-android-smok-18792` 正常触发 | **FALSIFIED** ✅ |
| H4 | Windows 桌面只能发不能收 | smoke 启动后立刻 `Discovered: android-smoke @ [192.168.5.130]` | **FALSIFIED** ✅ |
| H5 | 服务类型不匹配 | Rust `_pastebridge._tcp.local.` 与 Android `_pastebridge._tcp.` 经 RFC 6762 等价,且实测 Rust 成功解析到 Android 的 `PasteBridge-android-smok-18792._pastebridge._tcp.local.` | **FALSIFIED** ✅ |

### 关键时间线

- **16:13:32** Android app 启动,browse 启动 (`wlan0` 接口),`onServiceRegistered` 触发
- **16:15:08** smoke test #1 (30s 限时) 启动 → 成功发现 Android,Android **未**发现 desktop
- **16:16:30** smoke test #1 退出
- **16:20:21** Android NSD 接收 `_kdeconnect._udp` 响应 (说明 mDNS 双向通)
- **16:21:30** smoke test #3 (120s 限时) 启动 → 成功发现 Android,Android **仍未**发现 desktop
- **16:22:21** smoke test #4 启动,改用 `addresses=["192.168.5.226"]` (本机 wlan IP) → **Android 立即** 触发 `onServiceFound: PasteBridge-desktop-lapt-18792` + `onServiceResolved: desktop-laptop4 @ [192.168.5.226]:18792`

## 根因

**`mdns-sd` 0.17 + Windows Bonjour 行为：传 `0.0.0.0` 作为 A 记录时,Bonjour mDNSResponder 只在 loopback 注册,不会向网络通告。**

证据:
- smoke test 起初传 `addresses: Vec::new()` → 内部拼成 `"0.0.0.0"` → Bonjour 注册成功但只 loopback
- 同一进程内的 browse 能看到 (因为是 loopback) → smoke 自发现
- Android (跨网络) 看不到 → 0 个 mDNS 公告到 wlan
- 改成 `addresses: vec!["192.168.5.226"]` → Bonjour 在 wlan 接口上发公告 → Android 立刻发现

**这不影响生产代码**: `crates/desktop/src/main.rs:373` 已经调用 `list_local_ipv4()` 传真实物理接口 IP;`crates/mobile/.../MainActivity.kt:23` 的 NsdManager 由系统处理,会使用系统检测到的所有网络接口。

**影响的只是 `crates/core/examples/mdns_smoke.rs`** — 测试样例本身写错了,传空 vec。

## 修复

✅ **`crates/core/examples/mdns_smoke.rs`**: 改为 `vec!["192.168.5.226".to_string()]` (或读取本机接口的 IP,不要传 0.0.0.0)。

下一步可选: 把 smoke test 改成自动取 `list_local_ipv4()` 的第一个,这样跨机器跑测试不用改 IP。

## 验证 (post-fix)

```
[smoke] device_id = desktop-laptop4
[mdns] Registered: PasteBridge-desktop-lapt-18792._pastebridge._tcp.local. (host=DESKTOP-HMJ2UTI.local., port=18792, platform=smoke, device_id=desktop-lapt)
[smoke] running for 120s, Ctrl+C to exit early
[mdns] Discovered: android-smoke @ ["fdcb:5337:6614:0:1cbc:ccff:fef7:da91", "fe80::1cbc:ccff:fef7:da91", "192.168.5.130"]:18792 (platform=android)
[smoke] #1 discovered: device_id=android-smoke platform=android addrs=... port=18792
[mdns] Discovered: desktop-laptop4 @ ["192.168.5.226"]:18792 (platform=smoke)
[smoke] #2 discovered: device_id=desktop-laptop4 platform=smoke addrs=["192.168.5.226"] port=18792

# Android logcat
06-11 16:22:29.388 28068 28160 D PBDiscovery: onServiceFound: PasteBridge-desktop-lapt-18792
06-11 16:22:29.395 28068 28160 D PBDiscovery: onServiceResolved: desktop-laptop4 @ [192.168.5.226]:18792
06-11 16:22:29.396 28068 28068 D PBApp: discovered: desktop-laptop4 smoke @ [192.168.5.226]:18792
```

**双向发现已端到端跑通。**

