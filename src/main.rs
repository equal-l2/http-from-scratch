use std::io::{Read, Write};
use std::net;

mod parse;

fn handle_stream(stream: &mut net::TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 1024];
    let mut tail = 0;
    'main: loop {
        match stream.read(&mut buf[tail..]) {
            Ok(n) => {
                tail = tail + n - 1;
                if let Some(header_tail) = parse::utils::find_header_end(&buf[..=tail]) {
                    let parsed: Result<parse::RequestHeader, _> =
                        buf[..=(header_tail - 2)].try_into();
                    match parsed {
                        Ok(header) => {
                            eprintln!("{:#?}", header);
                            // TODO: read body using Content-Length
                        }
                        Err(e) => {
                            eprintln!("Handler error: {}", e);
                            bad_request(stream)?;
                            return Ok(());
                        }
                    }
                    ok(stream)?;
                    break 'main;
                } else if buf.len() == tail + 1 {
                    // buffer is filled up yet header is not found
                    eprintln!("Handler error: buffer is full but header is not found yet");
                    bad_request(stream)?;
                    return Ok(());
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn response(stream: &mut net::TcpStream, code: u16, mesg: &str) -> Result<(), std::io::Error> {
    write!(stream, "HTTP/1.1 {} {}", code, mesg)?;
    stream.write_all(b"\r\n\r\n")?;
    Ok(())
}

fn ok(stream: &mut net::TcpStream) -> Result<(), std::io::Error> {
    response(stream, 200, "OK")
}

fn bad_request(stream: &mut net::TcpStream) -> Result<(), std::io::Error> {
    response(stream, 400, "Bad Request")
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
