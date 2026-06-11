//! Quick TCP/HTTP probe from PC to a target address:port.
//! Prints whether connect succeeded and what (if anything) the HTTP server returns.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let target = args.next().unwrap_or_else(|| "10.239.38.89:18792".to_string());
    let method = args.next().unwrap_or_else(|| "GET".to_string());
    let path = args.next().unwrap_or_else(|| "/clipboard/history".to_string());

    eprintln!("[probe] target={} {} {}", target, method, path);
    let mut stream = match TcpStream::connect_timeout(
        &target.parse().expect("bad addr"),
        Duration::from_secs(3),
    ) {
        Ok(s) => {
            eprintln!("[probe] TCP connect OK ({:?})", s.local_addr());
            s
        }
        Err(e) => {
            eprintln!("[probe] TCP connect FAIL: {}", e);
            return;
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    stream.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
    let req = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        method, path, target
    );
    stream.write_all(req.as_bytes()).unwrap();
    eprintln!("[probe] sent {} bytes", req.len());
    let mut buf = Vec::new();
    let n = stream.read_to_end(&mut buf).unwrap_or_else(|e| {
        eprintln!("[probe] read err: {}", e);
        0
    });
    eprintln!("[probe] read {} bytes", n);
    let resp = String::from_utf8_lossy(&buf);
    println!("{}", resp);
}
