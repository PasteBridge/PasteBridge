# sync-with-device HTTP 闭环 Smoke Test

> 路径: `crates/core/examples/sync_with_device_smoke.rs`

## 跑法

```powershell
cargo run --example sync_with_device_smoke
```

期望 (在 stderr):

```
[smoke-sync] A_PORT=28900
[A] seeded count = 3
[B] initial count = 2
[B] calling sync_with_device(peer=A) ...
[B] report = pulled=3 added=3 pushed=1 err=None
[check] A.count=4 B.count=5
[smoke-sync] OK: pull + push closed loop verified
[smoke-sync] done
```

## 验证内容

1. A 启动 HTTP server,seed 3 条历史。
2. B (本地 2 条) 调 `sync_with_device(A_peer)`:
   - **Pull**: GET `127.0.0.1:28900/clipboard/history` → 解析 3 条 → 用 `content_hash` 与 B 本地比对 → 3 条新增到 B。
   - **Push**: 取 B 本地最新 ("from-B-2") → POST `127.0.0.1:28900/clipboard/copy` → A 通过 `ClipboardApiCallback::on_remote_copy` 落入历史。
3. 断言:
   - `pulled_total == 3`, `pulled_added == 3`, `pushed == 1`, `error == None`
   - B.history.len() == 5 (2 本地 + 3 拉取)
   - A.history 包含 B push 的 "from-B-2"

## 与桌面 UI 的对应

桌面侧 `crates/desktop/src/callbacks.rs::register_sync_with_device` 走
同一份 `paste_bridge_core::sync_device::sync_with_device`,只是额外:

- 把 HTTP 调用丢到 `std::thread`,不阻塞 Slint 主事件循环
- 通过 `slint::invoke_from_event_loop` 把 `SyncReport` 弹回主线程
- 拉到了新条目 (`pulled_added > 0`) 时触发 `sync_history_to_ui_async` 刷新列表
- 用 `slint::Timer::single_shot` 在 3.5s 后关闭 toast

mDNS 发现的远端 (Android 端走 UniFFI Rust core, 桌面端走 mdns-sd) 通过
`DiscoveredPeer { addresses, port, ... }` 直接传进 `sync_with_device`。
HTTP 协议字段对称, 桌面↔桌面 / 桌面↔Android 都能跑通同一份 sync 闭环。
