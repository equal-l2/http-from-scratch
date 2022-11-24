use std::collections::HashMap;

mod parse;

#[derive(Debug)]
pub struct RequestHeader<'a> {
    pub request_line: RequestLine<'a>,
    pub headers: HashMap<HeaderName, &'a str>,
}

#[derive(Debug)]
pub struct RequestLine<'a> {
    pub method: Method,
    pub path: &'a str,
    pub version: &'a str,
}

#[derive(Debug)]
pub enum Method {
    Get,
    Head,
    Post,
}

#[derive(Debug)]
pub struct Header<'a>(HeaderName, &'a str);

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum HeaderName {
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
