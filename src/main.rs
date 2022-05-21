use std::borrow::Cow;
use std::io::{Read, Write};
use std::net;

macro_rules! dump_bytestr {
    ($name: expr, $v: expr) => {
        eprintln!(
            "{}:\n{}\n!!!",
            $name,
            std::str::from_utf8($v).expect("valid utf-8 stream")
        )
    };

    ($v: expr) => {
        eprintln!(
            "ANONYMOUS:\n{}\n!!!",
            std::str::from_utf8($v).expect("valid utf-8 stream")
        )
    };
}

type Error = Box<dyn std::error::Error>;

#[derive(Debug)]
enum Method {
    Get,
    Head,
    Post,
}

impl TryFrom<&[u8]> for Method {
    type Error = ();
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        match s.to_ascii_lowercase().as_slice() {
            b"get" => Ok(Method::Get),
            b"head" => Ok(Method::Head),
            b"post" => Ok(Method::Post),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
struct RequestLine<'a> {
    method: Method,
    path: Cow<'a, str>,
    version: Cow<'a, str>,
}

#[derive(Debug)]
struct RequestLineParseError(String);

impl std::fmt::Display for RequestLineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse Request-Line: {}", self.0)
    }
}

impl From<&str> for RequestLineParseError {
    fn from(s: &str) -> Self {
        RequestLineParseError(s.into())
    }
}

impl<'a> TryFrom<&'a [u8]> for RequestLine<'a> {
    type Error = RequestLineParseError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let mut fields = data.split(|&ch| ch == b' ');
        let method = fields
            .next()
            .ok_or("Method not found")?
            .try_into()
            .map_err(|_| "Unsupported method")?;
        let path = fields.next().ok_or("Path not found")?;
        let version = fields.next().ok_or("Version not found")?;
        Ok(RequestLine {
            method,
            path: String::from_utf8_lossy(path),
            version: String::from_utf8_lossy(version),
        })
    }
}

#[derive(Debug)]
struct RequestHeader<'a> {
    request_line: RequestLine<'a>,
    headers: std::collections::HashMap<&'a str, &'a str>,
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    let pat_len = pattern.len();
    for idx in pat_len..data.len() {
        let window = &data[(idx - pat_len + 1)..=idx];
        if window == pattern {
            return Some(idx);
        }
    }
    None
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    find_pattern(data, b"\r\n\r\n")
}

fn find_newline(data: &[u8]) -> Option<usize> {
    find_pattern(data, b"\r\n")
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_find_pattern() {
        assert_eq!(find_pattern(b"hello world!!!", b"!"), Some(11));
        assert_eq!(find_pattern(b"hello world", b"!"), None);
    }

    #[test]
    fn test_find_new_line() {
        assert_eq!(find_newline(b"hello\r\nworld"), Some(6));
        assert_eq!(find_newline(b"hello world\r"), None);
        assert_eq!(find_newline(b"Header: localhost:8080\r\n"), Some(23));
    }
}

#[derive(Debug)]
enum RequestHeaderParseError {
    RequestLine(RequestLineParseError),
    Header(HeaderParseError),
}

fn parse_request_header(data: &[u8]) -> Result<RequestHeader, RequestHeaderParseError> {
    // SAFETY: data forms proper header

    let request_line_tail = find_newline(data).expect("newline must exist");
    let request_line = data[..=request_line_tail]
        .try_into()
        .map_err(RequestHeaderParseError::RequestLine)?;

    let mut head = request_line_tail + 1;
    let mut headers = std::collections::HashMap::new();
    loop {
        if head >= data.len() {
            break;
        }
        match find_newline(&data[head..]) {
            Some(local_tail) => {
                // local_tail is based on data[head..]
                let tail = head + local_tail;
                //dump_bytestr!("HEADER", &data[head..=tail]);
                let (name, value) =
                    parse_header(&data[head..=tail]).map_err(RequestHeaderParseError::Header)?;
                headers.insert(name, value);
                head = tail + 1;
            }
            None => unreachable!(),
        }
    }
    Ok(RequestHeader {
        request_line,
        headers,
    })
}

#[derive(Debug)]
enum HeaderParseError {
    NoSeparator,
    InvalidUtf8Name(std::str::Utf8Error),
    InvalidUtf8Value(std::str::Utf8Error),
}

fn parse_header(data: &[u8]) -> Result<(&str, &str), HeaderParseError> {
    // assume header name and value are UTF-8
    let split = data.splitn(2, |v| v == &b':').collect::<Vec<_>>();
    debug_assert!(split.len() <= 2);
    match split.as_slice() {
        [name_byte, value_byte] => {
            let name = std::str::from_utf8(name_byte).map_err(HeaderParseError::InvalidUtf8Name)?;
            let value =
                std::str::from_utf8(value_byte).map_err(HeaderParseError::InvalidUtf8Value)?;
            Ok((name, value))
        }
        _ => Err(HeaderParseError::NoSeparator),
    }
}

fn handle_stream(stream: &mut net::TcpStream) -> Result<(), Error> {
    let mut buf = [0u8; 1024];
    let mut tail = 0;
    'main: loop {
        match stream.read(&mut buf[tail..]) {
            Ok(n) => {
                tail = tail + n - 1;
                if let Some(header_tail) = find_header_end(&buf[..=tail]) {
                    match parse_request_header(&buf[..=(header_tail - 2)]) {
                        Ok(header) => {
                            eprintln!("{:?}", header);
                            // TODO: read body using Content-Length
                        }
                        Err(e) => {
                            eprintln!("Error: {:?}", e);
                            stream.shutdown(std::net::Shutdown::Read)?;
                            bad_request(stream)?;
                        }
                    }
                    ok(stream)?;
                    break 'main;
                } else if buf.len() == tail + 1 {
                    // buffer is filled up yet header is not found
                    eprintln!("Error: buffer is full but header is not found yet");
                    bad_request(stream)?;
                    return Ok(());
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn response(stream: &mut net::TcpStream, code: u16, mesg: &str) -> Result<(), Error> {
    write!(stream, "HTTP/1.1 {} {}", code, mesg)?;
    stream.write_all(b"\r\n\r\n")?;

    // TODO: keep-alive
    stream.shutdown(std::net::Shutdown::Both)?;

    Ok(())
}

fn ok(stream: &mut net::TcpStream) -> Result<(), Error> {
    response(stream, 200, "OK")
}

fn bad_request(stream: &mut net::TcpStream) -> Result<(), Error> {
    response(stream, 400, "Bad Request")
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
    let addr = "0.0.0.0:8080";
    eprintln!("listening on {}", addr);
    handle_listener(addr).unwrap();
}
