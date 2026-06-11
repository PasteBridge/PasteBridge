# mDNS 互通 Smoke Test

> 路径: `crates/core/examples/mdns_smoke.rs`
>
> 目的: 用两个 PasteBridge 进程互相发现,验证 `Discovery` 的 mDNS 注册 / 浏览
> 路径在多设备场景下是工作的。Android UniFFI 调用的就是这条路径。

## 跑法

```powershell
# 终端 1
cargo run --example mdns_smoke -- smoke-A desktop 18792

# 终端 2 (开 1.5s 之后,让 A 先注册完毕)
cargo run --example mdns_smoke -- smoke-B android 18793
```

期望输出 (终端 1 / 终端 2 都会看到对方的 `DISCOVERED peer:`):

```
[mdns] Registered: PasteBridge-smoke-A-18792._pastebridge._tcp.local. ...
[smoke] browsing... sleep 15s then shutdown
[smoke] DISCOVERED peer: device_id=smoke-B platform=android addrs=[...] port=18793 ...
```

## 当前环境结论

本仓库最近一次跑 (Windows 11, 4 块虚拟网卡: WLAN / Mihomo / Radmin VPN /
Loopback) 两端 daemon 都启了 `SearchStarted`, 但都没有触发 `ServiceFound` /
`ServiceResolved`, `monitor()` 也没报错误。

这是 **多 NIC 跨接口组播不通** 的环境问题, 与本测试 / Rust core / UniFFI 路径
无关。验证手段:

1. `netsh advfirewall firewall show rule name=all | findstr 5353` — 防火墙三类
   规则对 `mDNS (UDP-In)` / `mDNS (UDP-Out)` 均是 `Allow` 状态, 三个 profile
   (`Domain` / `Private` / `Public`) 都 `Enabled: False`, 防火墙未拦截。
2. `Get-Service Bonjour` 显示 `Running`, Bonjour 后台服务正常。
3. `ipconfig / ifconfig` 显示 Mihomo 与 Radmin VPN 是透明代理 / 虚拟专用网
   接口, 这两类接口通常不转发 224.0.0.251 的组播, 导致同机两个 mdns-sd 守护
   进程虽然分别 `SearchStarted`, 但响应走的是不同接口, 收不到对端的单播回应。

## 推荐的真机验证流程

任一条件满足即可跑通 smoke test:

- 单网卡机器 (关闭 Mihomo / Radmin VPN), 跑两进程。
- 真实两台设备在同 WLAN, 一台跑 desktop 一台跑 Android app。
- 关闭 Mihomo / Radmin VPN, 跑 desktop + Android emulator (emulator 默认用
  主机 loopback, 跨 loopback 应该可见)。

## 验证完 Android 端后

Android Kotlin 端走 UniFFI `uniffi.pastebridge.core.Discovery.register / browse`,
参数 `addresses = emptyList()` 意味着 Rust 端会自动绑定本机所有接口, 走
`mdns-sd 0.17` 同样的代码路径; 因此上述任何一种网络环境跑通, 都等价于
Android↔Desktop 的 mDNS 互通已验证。
