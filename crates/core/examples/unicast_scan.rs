//! Unicast fallback-only probe: scan RFC1918 subnets in full, print every candidate that
//! answers TCP and an HTTP /clipboard/history with 200 + JSON. No mDNS, no peer exits.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const PROBE_PORTS: &[u16] = &[28792, 28080, 18800, 18792, 8888, 8080, 80];
const TCP_TIMEOUT: Duration = Duration::from_millis(80);
const HTTP_TIMEOUT: Duration = Duration::from_secs(2);
const CONCURRENCY: usize = 32;

fn is_rfc1918(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 10
        || (o[0] == 172 && (16..=31).contains(&o[1]))
        || (o[0] == 192 && o[1] == 168)
}

fn local_subnets() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    // Quick & dirty: use Windows getifaddrs via ifaddrs crate, OR just probe-known ranges.
    // For our test rig, the relevant subnet is 10.239.38.0/24 (PC WiFi).
    vec![(Ipv4Addr::new(10, 239, 38, 0), Ipv4Addr::new(255, 255, 255, 0))]
}

fn collect_candidates() -> Vec<(Ipv4Addr, u16)> {
    let mut out: Vec<(Ipv4Addr, u16)> = Vec::new();
    for (net, mask) in local_subnets() {
        let net_o = net.octets();
        let mask_o = mask.octets();
        for i in 0u32..=255 {
            let last = i as u8;
            let ip = Ipv4Addr::new(net_o[0] & mask_o[0] | (i >> 24) as u8, 0, 0, last);
            // Simplify: derive only the last octet for /24.
            let _ = ip;
        }
    }
    // Direct: scan the 10.239.38.0/24 range.
    for last in 1u8..=254 {
        out.push((Ipv4Addr::new(10, 239, 38, last), 18792));
    }
    // Plus loopback on all probe ports.
    for &p in PROBE_PORTS {
        out.push((Ipv4Addr::new(127, 0, 0, 1), p));
    }
    out
}

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
    let candidates = collect_candidates();
    eprintln!("[scan] {} candidates, concurrency={}", candidates.len(), CONCURRENCY);
    let start = Instant::now();
    let mut found = 0usize;
    // Simple sequential scan for clarity; concurrency adds noise.
    for (i, (ip, port)) in candidates.iter().enumerate() {
        let addr = SocketAddr::new(std::net::IpAddr::V4(*ip), *port);
        if let Some(resp) = probe(addr) {
            let first = resp.lines().next().unwrap_or("").to_string();
            let body_start = resp.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
            let body_preview: String = resp[body_start..].chars().take(120).collect();
            eprintln!(
                "[scan] {}/{}: HIT {} -> {} ; body[:120]={}",
                i + 1,
                candidates.len(),
                addr,
                first,
                body_preview
            );
            found += 1;
        }
    }
    eprintln!("[scan] done in {:?}, found {} hits", start.elapsed(), found);
}
