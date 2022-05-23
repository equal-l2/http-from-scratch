use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
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
        dump_bytestr!("ANONYMOUS", $v)
    };
}

type Error = Box<dyn std::error::Error>;

#[derive(Debug)]
enum Method {
    Get,
    Head,
    Post,
}

#[derive(Debug, Hash, PartialEq, Eq)]
enum HeaderName {
    Accept,
    Connection,
    ContentLength,
    Host,
    Referer,
    UserAgent,
    Other(String),
}
// TODO: parse from &[u8] instead of from &str?

impl std::str::FromStr for HeaderName {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        Ok(match lower.as_str() {
            "accept" => Self::Accept,
            "connection" => Self::Connection,
            "content-length" => Self::ContentLength,
            "host" => Self::Host,
            "referer" => Self::Referer,
            "user-agent" => Self::UserAgent,
            _ => Self::Other(lower),
        })
    }
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
enum RequestLineParseError {
    MissingMethod,
    UnsupportedMethod,
    MissingPath,
    MissingVersion,
}

impl std::fmt::Display for RequestLineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMethod => write!(f, "method was not found"),
            Self::UnsupportedMethod => write!(f, "method was unsupported"),
            Self::MissingPath => write!(f, "path was not found"),
            Self::MissingVersion => write!(f, "version was not found"),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for RequestLine<'a> {
    type Error = RequestLineParseError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let mut fields = data.split(|&ch| ch == b' ');
        let method_bytes = fields.next().ok_or(Self::Error::MissingMethod)?;
        let method = method_bytes
            .try_into()
            .map_err(|_| Self::Error::UnsupportedMethod)?;
        let path = fields.next().ok_or(Self::Error::MissingPath)?;
        let version = fields.next().ok_or(Self::Error::MissingVersion)?;
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
    headers: HashMap<HeaderName, &'a str>,
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

impl std::fmt::Display for RequestHeaderParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestLine(e) => write!(f, "failed to parse Request-Line: {}", e),
            Self::Header(e) => write!(f, "failed to parse Header: {}", e),
        }
    }
}

fn parse_request_header(data: &[u8]) -> Result<RequestHeader, RequestHeaderParseError> {
    // SAFETY: data forms proper header

    let request_line_tail = find_newline(data).expect("newline must exist");
    let request_line_bytes = &data[..=(request_line_tail - 2)]; // drop CRLF
    let request_line = request_line_bytes
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
                let header_bytes = &data[head..=(tail - 2)]; // drop CRLF
                let (kind, value) =
                    parse_header(header_bytes).map_err(RequestHeaderParseError::Header)?;
                let value = value.trim(); // drop OWS
                headers.insert(kind, value);
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
    InvalidUtf8Name,
    InvalidUtf8Value,
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSeparator => write!(f, "separator colon was missing"),
            Self::InvalidUtf8Name => write!(f, "name was invalid UTF-8",),
            Self::InvalidUtf8Value => write!(f, "value was invalid UTF-8",),
        }
    }
}

fn parse_header(data: &[u8]) -> Result<(HeaderName, &str), HeaderParseError> {
    // assume data is proper header, and header name and value are UTF-8
    let split = data.splitn(2, |v| v == &b':').collect::<Vec<_>>();
    debug_assert!(split.len() <= 2);
    match split.as_slice() {
        [name_bytes, value_bytes] => {
            let name = std::str::from_utf8(name_bytes)
                .map_err(|_| HeaderParseError::InvalidUtf8Name)?;
            let value = std::str::from_utf8(value_bytes)
                .map_err(|_| HeaderParseError::InvalidUtf8Value)?;
            Ok((name.parse().unwrap(), value))
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
                            eprintln!("{:#?}", header);
                            // TODO: read body using Content-Length
                        }
                        Err(e) => {
                            eprintln!("Handler error: {}", e);
                            bad_request(stream)?;
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

fn response(stream: &mut net::TcpStream, code: u16, mesg: &str) -> Result<(), Error> {
    write!(stream, "HTTP/1.1 {} {}", code, mesg)?;
    stream.write_all(b"\r\n\r\n")?;
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
