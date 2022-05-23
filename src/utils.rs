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

pub fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    let pat_len = pattern.len();
    for idx in pat_len..data.len() {
        let window = &data[(idx - pat_len + 1)..=idx];
        if window == pattern {
            return Some(idx);
        }
    }
    None
}

pub fn find_header_end(data: &[u8]) -> Option<usize> {
    find_pattern(data, b"\r\n\r\n")
}

pub fn find_newline(data: &[u8]) -> Option<usize> {
    find_pattern(data, b"\r\n")
}

pub fn is_tchar(ch: u8) -> bool {
    matches!(ch,
        0x21 | // !
        0x23..=0x27 | // # $ % & '
        0x2A | // *
        0x2B | // +
        0x2D | // -
        0x2E | // .
        0x30..=0x39 | // 0-9
        0x41..=0x5A | // A-Z
        0x5E..=0x60 | // ^ _ `
        0x61..=0x7A | // & a-z
        0x7C | // |
        0x7E // ~
    )
}

pub fn is_token(data: &[u8]) -> bool {
    data.iter().all(|&c| is_tchar(c))
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
