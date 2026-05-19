//! HTTP API Server

use std::sync::{Arc, mpsc};
use std::thread;
use std::io::Read;
use tiny_http::{Server, Response};
use crate::core::state::AppState;
use crate::api::routes;

pub struct ApiServer {
    port: u16,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl ApiServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            shutdown_tx: None,
        }
    }

    /// Start the API server
    pub fn start(&mut self, state: Arc<AppState>) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Server::http(&addr)
            .map_err(|e| format!("Failed to start server on {}: {}", addr, e))?;
        
        eprintln!("[api] API server listening on http://{}", addr);

        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let state_clone = state.clone();
        thread::spawn(move || {
            for request in server.incoming_requests() {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                let method = request.method().as_str().to_string();
                let path = request.url().to_string();
                let response = handle_request(&method, &path, &state_clone);
                let _ = request.respond(response);
            }
        });

        Ok(())
    }

    /// Stop the API server
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

fn handle_request(
    method: &str,
    path: &str,
    state: &Arc<AppState>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    eprintln!("[api] {} {}", method, path);

    match (method, path) {
        ("GET", "/clipboard/history") => {
            let body = routes::handle_get_history(state);
            Response::from_data(body)
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()
                )
        }

        ("POST", "/clipboard/copy") => {
            let body = routes::handle_copy(state, &[]);
            match body {
                Ok(_) => Response::from_string("OK"),
                Err(e) => Response::from_string(format!("Error: {}", e))
                    .with_status_code(400),
            }
        }

        ("POST", "/clipboard/clear") => {
            routes::handle_clear(state);
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
