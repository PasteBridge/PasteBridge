//! Concurrent unicast scan: spawn N tasks, each probes a slice of /24 IPs on a single port.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const TCP_TIMEOUT: Duration = Duration::from_millis(80);
const HTTP_TIMEOUT: Duration = Duration::from_millis(400);

fn probe(addr: SocketAddr) -> Option<String> {
    let mut s = TcpStream::connect_timeout(&addr, TCP_TIMEOUT).ok()?;
    s.set_read_timeout(Some(HTTP_TIMEOUT)).ok()?;
    s.set_write_timeout(Some(HTTP_TIMEOUT)).ok()?;
    let req = format!(
        "GET /clipboard/history HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        addr
    );
    s.write_all(req.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let _ = s.read_to_end(&mut buf);
    let resp = String::from_utf8_lossy(&buf).to_string();
    if resp.contains("HTTP/1.1 200") || resp.starts_with("HTTP/1.0 200") {
        Some(resp)
    } else {
        None
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let subnet: String = args.next().unwrap_or_else(|| "10.239.38".to_string());
    let port: u16 = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18792);
    let concurrency: usize = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);

    let parts: Vec<u8> = subnet
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    if parts.len() != 3 {
        eprintln!("[scan] subnet must be a.b.c");
        return;
    }
    let (a, b, c) = (parts[0], parts[1], parts[2]);
    let candidates: Vec<SocketAddr> = (1u8..=254)
        .map(|last| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, last)), port))
        .collect();
    eprintln!(
        "[scan] subnet={}.0/24 port={} cand={} conc={}",
        subnet,
        port,
        candidates.len(),
        concurrency
    );

    let hits: Arc<Mutex<Vec<(SocketAddr, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let start = Instant::now();
    let mut handles = Vec::new();
    let chunk = (candidates.len() + concurrency - 1) / concurrency;
    for slice in candidates.chunks(chunk) {
        let slice = slice.to_vec();
        let hits = hits.clone();
        let h = thread::spawn(move || {
            for addr in slice {
                if let Some(resp) = probe(addr) {
                    let first = resp.lines().next().unwrap_or("").to_string();
                    let body_start = resp.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
                    let body_preview: String = resp[body_start..].chars().take(120).collect();
                    eprintln!(
                        "[scan] HIT {} -> {} ; body[:120]={}",
                        addr, first, body_preview
                    );
                    hits.lock().unwrap().push((addr, first));
                }
            }
        });
        handles.push(h);
    }
    for h in handles {
        h.join().unwrap();
    }
    let hits = hits.lock().unwrap();
    eprintln!(
        "[scan] done in {:?}, found {} hits",
        start.elapsed(),
        hits.len()
    );
    for (addr, first) in hits.iter() {
        println!("{} -> {}", addr, first);
    }
}
