use std::collections::HashMap;

mod parse;

#[derive(Debug)]
pub struct RequestHeader<'a> {
    request_line: RequestLine<'a>,
    headers: HashMap<HeaderName, &'a str>,
}

#[derive(Debug)]
pub struct RequestLine<'a> {
    method: Method,
    path: &'a str,
    version: &'a str,
}

#[derive(Debug)]
enum Method {
    Get,
    Head,
    Post,
}

#[derive(Debug)]
pub struct Header<'a>(HeaderName, &'a str);

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

pub struct Response {
    headers: HashMap<HeaderName, String>,
    body: Vec<u8>,
}
