#[derive(Debug)]
pub enum RequestLineParseError {
    Method,
    Path,
    Version,
}

impl std::fmt::Display for RequestLineParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Method => write!(f, "method was missing or invalid"),
            Self::Path => write!(f, "path was missing or invalid"),
            Self::Version => write!(f, "version was missing or invalid"),
        }
    }
}

#[derive(Debug)]
pub enum RequestHeaderParseError {
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

#[derive(Debug)]
pub enum HeaderParseError {
    NoSeparator,
    InvalidName,
    InvalidUtf8Value,
}

impl std::fmt::Display for HeaderParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSeparator => write!(f, "separator colon was missing"),
            Self::InvalidName => write!(f, "name contains non-tchar character",),
            Self::InvalidUtf8Value => write!(f, "value was invalid UTF-8",),
        }
    }
}

pub struct HeaderNameParseError;

impl std::fmt::Display for HeaderNameParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "contains non-tchar character")
    }
}
