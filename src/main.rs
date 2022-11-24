use std::io::{Read, Write};
use std::net;

mod types;
mod utils;

use types::*;

fn handle_stream(stream: &mut net::TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 1024];
    let mut tail = 0;
    'main: loop {
        match stream.read(&mut buf[tail..]) {
            Ok(0) => {
                eprintln!("Handler error: unexpected end of the request");
                bad_request(stream, &[])?;
                break 'main;
            }
            Ok(n) => {
                tail = tail + n;
                if let Some(header_tail) = utils::find_header_end(&buf[..=tail]) {
                    let parsed: Result<types::RequestHeader, _> =
                        buf[..=(header_tail - 2)].try_into();
                    match parsed {
                        Ok(header) => {
                            eprintln!("{header:?}");
                            // Read body
                            let body = {
                                if let Some(len_str) =
                                    header.headers.get(&HeaderName::ContentLength)
                                {
                                    let len: usize = len_str.parse()?;
                                    let body = &buf[(header_tail + 1)..=(header_tail + len)];
                                    Some(body)
                                } else {
                                    None
                                }
                            };

                            let RequestLine { method, path, .. } = header.request_line;

                            match method {
                                Method::Get => match path {
                                    "/" =>
                                        ok(stream, b"<!doctype html><html><head><title>GET RESPONSE</title><body>Hello world!</body></html>")?,
                                    _ => response(stream, 404, "Not Found", &[])?,
                                },
                                _ => response(stream, 405, "Method Not Allowed", &[])?,
                            }
                        }
                        Err(e) => {
                            eprintln!("Handler error: {}", e);
                            bad_request(stream, &[])?;
                        }
                    }
                    break 'main;
                } else if buf.len() == tail + 1 {
                    eprintln!("Handler error: buffer is full but header is not found yet");
                    bad_request(stream, &[])?;
                    break 'main;
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn response(
    stream: &mut net::TcpStream,
    code: u16,
    mesg: &str,
    body: &[u8],
) -> Result<(), std::io::Error> {
    write!(stream, "HTTP/1.0 {} {}\r\n", code, mesg)?;
    stream.write_all(b"connection: close\r\n")?;
    stream.write_all(b"\r\n\r\n")?;
    stream.write_all(body)?;
    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}

fn ok(stream: &mut net::TcpStream, body: &[u8]) -> Result<(), std::io::Error> {
    response(stream, 200, "OK", body)
}

fn bad_request(stream: &mut net::TcpStream, body: &[u8]) -> Result<(), std::io::Error> {
    response(stream, 400, "Bad Request", body)
}

fn handle_listener<T: net::ToSocketAddrs>(addr: T) -> Result<(), Box<dyn std::error::Error>> {
    let listener = net::TcpListener::bind(addr)?;

    for stream in listener.incoming() {
        let mut stream = stream?;
        if let Err(e) = handle_stream(&mut stream) {
            eprintln!("Stream error: {}", e);
        }
    }

    Ok(())
}

fn main() {
    let addr = "0.0.0.0:8080";
    eprintln!("listening on {}", addr);
    handle_listener(addr).unwrap();
}
