use std::sync::{Arc, mpsc};
use std::thread;
use tiny_http::{Server, Response};
use crate::state::AppState;
use crate::models::PasteBridgeError;
use crate::api::routes::{self, ClipboardApiCallback};

/// 跨平台 HTTP API 入口。
///
/// 桌面端走 `start_with_state(Arc<AppState>)`,Android/iOS 端通过 UniFFI
/// 实现 `ClipboardApiCallback` 后走 `start_with_callbacks(Box<dyn ...>)`。
#[derive(uniffi::Object)]
pub struct ApiServer {
    port: u16,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

// ===== FFI 可见的接口 =====
//
// 注意:
// - 任何 FFI 可见方法的参数必须满足 UniFFI 转换器边界 (`FfiConverter` /
//   `FfiConverterArc` / callback interface 的 `Box<dyn Trait>`)。
// - Rust-only 的 `start_with_state` 拆到下方不带 `#[uniffi::export]` 的
//   impl 块,避免编译器去尝试给 `Arc<AppState>` 找 FFI 转换器。
#[uniffi::export]
impl ApiServer {
    /// 用端口构造一个未启动的 server。
    #[uniffi::constructor]
    pub fn new(port: u16) -> Arc<Self> {
        Arc::new(Self {
            port,
            shutdown_tx: None,
        })
    }

    /// 移动端 (Android/iOS) 用的入口: 通过 UniFFI callback interface 拿到
    /// 历史 / 剪贴板推送 / 窗口可见性,不再依赖 `Arc<AppState>` 这种
    /// FFI 不好导出的 Rust 内部类型。
    pub fn start_with_callbacks(
        &self,
        callbacks: Box<dyn ClipboardApiCallback>,
    ) -> Result<(), PasteBridgeError> {
        let addr = format!("0.0.0.0:{}", self.port);
        let server = Server::http(&addr)
            .map_err(|e| PasteBridgeError::generic(format!("Failed to start server on {}: {}", addr, e)))?;

        eprintln!("[api] API server (callback-mode) listening on http://{}", addr);

        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        // shutdown_tx 暂存到 self(只在桌面端路径有意义,移动端忽略 stop 语义)
        drop(shutdown_tx);

        thread::spawn(move || {
            for mut request in server.incoming_requests() {
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                let method = request.method().as_str().to_string();
                let path = request.url().to_string();
                // 同步读 POST body, 失败当空 body 处理
                let mut body: Vec<u8> = Vec::new();
                let _ = std::io::Read::read_to_end(request.as_reader(), &mut body);
                let response = handle_request_cb(&method, &path, &body, &*callbacks);
                let _ = request.respond(response);
            }
        });

        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

// ===== Rust-only 接口 (不导出到 FFI) =====

impl ApiServer {
    /// 桌面端用的入口: 把 `Arc<AppState>` 直接当后端。
    /// 保持原签名,不动 desktop/main.rs 的调用风格。
    pub fn start_with_state(&self, state: Arc<AppState>) -> Result<(), String> {
        // 绑 0.0.0.0 让局域网内的其他设备(Android/iOS 端)能直接访问 API。
        // 之前绑 127.0.0.1 时 mDNS 公布的是 LAN IP 但 API 只在 loopback,导致
        // 其他设备按 mDNS 拿到的 IP + 端口做 TCP 探测时被 RST 拒绝,会误判设备离线。
        let addr = format!("0.0.0.0:{}", self.port);
        let server = Server::http(&addr)
            .map_err(|e| format!("Failed to start server on {}: {}", addr, e))?;

        eprintln!("[api] API server listening on http://{}", addr);

        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        // 桌面端由 main.rs 通过 stop() 触发;这里保留 sender 占位语义,
        // 不存到 self 以避免与 callback-mode 共享状态造成歧义。
        // (原 stop 通道语义在桌面端实际是空操作,因为 request loop 跑得快,
        // 进程退出时由 OS 回收 fd。)
        drop(shutdown_tx);

        let state_clone = state.clone();
        thread::spawn(move || {
            for mut request in server.incoming_requests() {
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                let method = request.method().as_str().to_string();
                let path = request.url().to_string();
                // POST 请求体在这里一次性读完。
                // GET/DELETE 走空 body 路径, 不会 block。
                let mut body: Vec<u8> = Vec::new();
                let _ = std::io::Read::read_to_end(request.as_reader(), &mut body);
                let response = handle_request(&method, &path, &body, &state_clone);
                let _ = request.respond(response);
            }
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

fn handle_request(
    method: &str,
    path: &str,
    body: &[u8],
    state: &Arc<AppState>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    eprintln!("[api] {} {} ({} bytes)", method, path, body.len());

    match (method, path) {
        ("GET", "/clipboard/history") => {
            let body = routes::handle_get_history(state);
            Response::from_data(body)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
                )
        }

        ("POST", "/clipboard/copy") => {
            let result = routes::handle_copy(state, body);
            match result {
                Ok(_) => Response::from_string("OK"),
                Err(e) => Response::from_string(format!("Error: {}", e))
                    .with_status_code(400),
            }
        }

        ("POST", "/clipboard/clear") => {
            let _ = routes::handle_clear(state);
            Response::from_string("OK")
        }

        ("GET", "/window/visible") => {
            let body = routes::handle_get_visible(state);
            Response::from_data(body)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
                )
        }

        ("POST", "/window/show") => {
            match routes::handle_window_show(state) {
                Ok(_) => Response::from_string("OK"),
                Err(e) => Response::from_string(format!("Error: {}", e)).with_status_code(400),
            }
        }

        ("POST", "/window/hide") => {
            match routes::handle_window_hide(state) {
                Ok(_) => Response::from_string("OK"),
                Err(e) => Response::from_string(format!("Error: {}", e)).with_status_code(400),
            }
        }

        _ => {
            Response::from_string("Not Found")
                .with_status_code(404)
        }
    }
}

fn handle_request_cb(
    method: &str,
    path: &str,
    body: &[u8],
    cb: &dyn ClipboardApiCallback,
) -> Response<std::io::Cursor<Vec<u8>>> {
    eprintln!("[api] (cb) {} {} ({} bytes)", method, path, body.len());

    match (method, path) {
        ("GET", "/clipboard/history") => {
            let body = routes::handle_get_history_cb(cb);
            Response::from_data(body).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            )
        }
        ("POST", "/clipboard/copy") => match routes::handle_copy_cb(cb, body) {
            Ok(_) => Response::from_string("OK"),
            Err(e) => Response::from_string(format!("Error: {}", e)).with_status_code(400),
        },
        ("POST", "/clipboard/clear") => {
            let _ = routes::handle_clear_cb(cb);
            Response::from_string("OK")
        }
        ("GET", "/window/visible") => {
            let body = routes::handle_get_visible_cb(cb);
            Response::from_data(body).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
            )
        }
        ("POST", "/window/show") => match routes::handle_window_show_cb(cb) {
            Ok(_) => Response::from_string("OK"),
            Err(e) => Response::from_string(format!("Error: {}", e)).with_status_code(400),
        },
        ("POST", "/window/hide") => match routes::handle_window_hide_cb(cb) {
            Ok(_) => Response::from_string("OK"),
            Err(e) => Response::from_string(format!("Error: {}", e)).with_status_code(400),
        },
        _ => Response::from_string("Not Found").with_status_code(404),
    }
}
