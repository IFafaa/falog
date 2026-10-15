//! Just enough URL encoding for OAuth query strings.

/// Percent-encodes everything except RFC 3986 unreserved characters.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Decodes `%XX` escapes and `+` (form encoding); malformed escapes are kept as they are.
pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' => match (bytes.get(i + 1).and_then(hex), bytes.get(i + 2).and_then(hex)) {
                (Some(high), Some(low)) => {
                    out.push(high << 4 | low);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(digit: &u8) -> Option<u8> {
    (*digit as char).to_digit(16).map(|value| value as u8)
}

/// `key=value` pairs of a query string, decoded.
pub fn query_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let text = "https://www.googleapis.com/auth/calendar.readonly ação";
        assert_eq!(decode(&encode(text)), text);
        assert_eq!(encode("a b/c"), "a%20b%2Fc");
    }

    #[test]
    fn keeps_malformed_escapes() {
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz"), "%zz");
        assert_eq!(decode("a+b%2B"), "a b+");
    }

    #[test]
    fn encodes_bytes_and_keeps_unreserved_characters() {
        assert_eq!(encode("ação"), "a%C3%A7%C3%A3o");
        assert_eq!(encode("AZaz09-._~"), "AZaz09-._~");
        assert_eq!(encode("a+b=c&d"), "a%2Bb%3Dc%26d");
    }

    #[test]
    fn decodes_either_case_and_replaces_invalid_text() {
        assert_eq!(decode("%2f%2F"), "//");
        assert_eq!(decode("%FF"), "\u{FFFD}");
        assert_eq!(decode("%4"), "%4");
    }

    #[test]
    fn query_pairs_skip_empty_parts_and_allow_bare_keys() {
        assert_eq!(
            query_pairs("&&flag&a+b=c+d&"),
            vec![("flag".into(), String::new()), ("a b".into(), "c d".into())]
        );
        assert!(query_pairs("").is_empty());
    }

    #[test]
    fn splits_queries() {
        let pairs = query_pairs("code=4%2F0Ab&state=xyz&scope=");
        assert_eq!(
            pairs,
            vec![
                ("code".into(), "4/0Ab".into()),
                ("state".into(), "xyz".into()),
                ("scope".into(), String::new()),
            ]
        );
    }
}
