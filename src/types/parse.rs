mod error;
use error::*;

use crate::{types::*, utils};

impl<'a> TryFrom<&'a [u8]> for RequestHeader<'a> {
    type Error = RequestHeaderParseError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // SAFETY: data forms proper header
        let request_line_tail = utils::find_newline(data).expect("newline must exist");
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
            match utils::find_newline(&data[head..]) {
                Some(local_tail) => {
                    // local_tail is based on data[head..]
                    let tail = head + local_tail;
                    let header_bytes = &data[head..=(tail - 2)]; // drop CRLF
                    let Header(kind, value) = header_bytes
                        .try_into()
                        .map_err(RequestHeaderParseError::Header)?;
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
}

impl<'a> TryFrom<&'a [u8]> for RequestLine<'a> {
    type Error = error::RequestLineParseError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let mut fields = data.split(|&ch| ch == b' ');
        let method_bytes = fields.next().ok_or(Self::Error::Method)?;
        let method = method_bytes.try_into().map_err(|_| Self::Error::Method)?;
        let path = fields.next().ok_or(Self::Error::Path)?;
        let version = fields.next().ok_or(Self::Error::Version)?;
        Ok(RequestLine {
            method,
            path: std::str::from_utf8(path).map_err(|_| Self::Error::Path)?,
            version: std::str::from_utf8(version).map_err(|_| Self::Error::Version)?,
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

impl<'a> TryFrom<&'a [u8]> for Header<'a> {
    type Error = HeaderParseError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let split = data.splitn(2, |v| v == &b':').collect::<Vec<_>>();
        // assume name and value are valid UTF-8
        match split.as_slice() {
            [name_bytes, value_bytes] => {
                let name = (*name_bytes)
                    .try_into()
                    .map_err(|_| HeaderParseError::InvalidName)?;
                let value = std::str::from_utf8(value_bytes)
                    .map_err(|_| HeaderParseError::InvalidUtf8Value)?;
                Ok(Header(name, value))
            }
            _ => Err(HeaderParseError::NoSeparator),
        }
    }
}

impl TryFrom<&[u8]> for HeaderName {
    type Error = HeaderNameParseError;
    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let lower = data.to_ascii_lowercase();
        Ok(match lower.as_slice() {
            b"accept" => Self::Accept,
            b"connection" => Self::Connection,
            b"content-length" => Self::ContentLength,
            b"host" => Self::Host,
            b"referer" => Self::Referer,
            b"user-agent" => Self::UserAgent,
            _ => {
                // check if data only contains tchars
                if !utils::is_token(&lower) {
                    return Err(HeaderNameParseError);
                } else {
                    // SAFETY: lower only contains tchar
                    let s = unsafe { String::from_utf8_unchecked(lower) };
                    Self::Other(s)
                }
            }
        })
    }
}
