//! Minimal HTTP server example: serves a hardcoded /clipboard/history response.
//! Used on the Android emulator to test E2E sync without NDK UniFFI build.

use std::io::Read;
use std::thread;
use std::time::Duration;

const RESPONSE_BODY: &str = r#"[{"id":99,"content_type":"text","content_text":"HELLO-FROM-ANDROID","content_hash":"abc99def","mime_type":null,"file_size":null,"width":null,"height":null,"source_ip":null,"created_at":1700000000000,"is_favorite":false},{"id":100,"content_type":"text","content_text":"test-from-phone-2","content_hash":"abc100def","mime_type":null,"file_size":null,"width":null,"height":null,"source_ip":null,"created_at":1700000060000,"is_favorite":false}]"#;

fn main() {
    let mut args = std::env::args().skip(1);
    let port: u16 = args.next().and_then(|s| s.parse().ok()).unwrap_or(18792);
    eprintln!("[http-smoke] listening on 0.0.0.0:{}", port);

    let server = match tiny_http::Server::http(("0.0.0.0", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[http-smoke] bind failed: {}", e);
            std::process::exit(1);
        }
    };

    for mut req in server.incoming_requests() {
        let method = req.method().as_str().to_string();
        let path = req.url().to_string();
        let mut body = Vec::new();
        let _ = req.as_reader().read_to_end(&mut body);
        eprintln!(
            "[http-smoke] {} {} body={}B",
            method,
            path,
            body.len()
        );
        let response = tiny_http::Response::from_string(RESPONSE_BODY)
            .with_header(
                "Content-Type: application/json"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            )
            .with_header(
                "Access-Control-Allow-Origin: *"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            );
        let _ = req.respond(response);
    }
    // Unreachable.
    drop(thread::sleep(Duration::from_secs(0)));
}