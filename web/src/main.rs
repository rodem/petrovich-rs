//! `petrovich-web` binary: `TcpListener` + thread per connection, no async.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use petrovich_web::{parse_query, reason, route};

fn handle_connection(mut stream: TcpStream) {
    let mut buf = vec![0u8; 16384];
    let mut request = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                request.extend_from_slice(&buf[..n]);
                if request.len() > 65536 {
                    return;
                }
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
        }
    }
    let head = String::from_utf8_lossy(&request);
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut words = request_line.split_whitespace();
    let (method, target) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let response = if method != "GET" {
        petrovich_web::Response {
            status: 404,
            content_type: "text/plain; charset=utf-8",
            body: "only GET is supported".to_owned(),
        }
    } else {
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        match parse_query(query) {
            Ok(params) => route(path, &params),
            Err(message) => petrovich_web::Response {
                status: 400,
                content_type: "text/plain; charset=utf-8",
                body: message,
            },
        }
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        reason(response.status),
        response.content_type,
        response.body.len(),
        response.body
    );
}

fn bind_address() -> String {
    std::env::args()
        .skip_while(|arg| arg != "--bind")
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_owned())
}

fn main() {
    let address = bind_address();
    let listener = TcpListener::bind(&address).unwrap_or_else(|e| {
        eprintln!("petrovich-web: cannot bind {address}: {e}");
        std::process::exit(1);
    });
    eprintln!("petrovich-web: listening on {address}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| handle_connection(stream));
            }
            Err(e) => eprintln!("petrovich-web: accept error: {e}"),
        }
    }
}
