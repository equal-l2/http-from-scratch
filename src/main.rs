use std::borrow::Cow;
use std::io::{Read, Write};
use std::net;

type Error = Box<dyn std::error::Error>;

// TODO: implement proper error
#[derive(Debug)]
struct MyError(&'static str);

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MyError {}

#[derive(Debug)]
enum Method {
    Get,
}

#[derive(Debug)]
struct Header<'a> {
    method: Method,
    path: Cow<'a, str>,
    version: Cow<'a, str>,
}

fn find_newline(data: &[u8]) -> Option<usize> {
    for index in 0..data.len() {
        if data[index] == b'\r' {
            if index + 1 < data.len() && data[index + 1] == b'\n' {
                return Some(index);
            }
        }
    }
    None
}

fn parse_header(data: &[u8]) -> Result<Header, Error> {
    if let Some(end) = find_newline(data) {
        let start_line = &data[0..end];
        let mut fields = start_line.split(|&ch| ch == b' ');
        let method = fields.next().ok_or(MyError("Method not found"))?;
        let method = match method.to_ascii_lowercase().as_slice() {
            b"get" => Method::Get,
            _ => return Err(MyError("Unsupported method").into()),
        };
        let path = fields.next().ok_or(MyError("Path not found"))?;
        let version = fields.next().ok_or(MyError("Version not found"))?;
        Ok(Header {
            method,
            path: String::from_utf8_lossy(path),
            version: String::from_utf8_lossy(version),
        })
    } else {
        // TODO: retrieve the rest of request data
        Err(MyError("Unexpected end of header").into())
    }
}

fn handle_stream(stream: &mut net::TcpStream) -> Result<(), Error> {
    let mut buffer = [0u8; 1024];
    stream.read(&mut buffer)?;
    match parse_header(&buffer) {
        Ok(header) => {
            println!("{:?}", header);
            // TODO: use header content to send proper response
            ok(stream)?;
        }
        Err(e) => {
            println!("Header error: {}", e);
            bad_request(stream)?;
        }
    }
    Ok(())
}

fn ok(stream: &mut net::TcpStream) -> Result<(), Error> {
    stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n")?;
    Ok(())
}

fn bad_request(stream: &mut net::TcpStream) -> Result<(), Error> {
    stream.write_all(b"HTTP/1.1 500 Bad Request\r\n\r\n")?;
    Ok(())
}

fn handle_listener<T: net::ToSocketAddrs>(addr: T) -> Result<(), Error> {
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
    handle_listener("0.0.0.0:8080").unwrap();
}
