// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 oxml. All rights reserved.

//! A minimal JSON reader and writer.
//!
//! MCP speaks JSON-RPC over stdio. The messages involved are small and
//! their shapes are fixed, so this implements the subset needed rather
//! than taking a serialisation dependency — which for a server meant
//! to be audited is a worthwhile trade.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`
    Null,
    /// `true` or `false`
    Bool(bool),
    /// A number.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object. Ordered so output is deterministic.
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// Build an object from pairs.
    #[must_use]
    pub fn object(pairs: Vec<(&str, Self)>) -> Self {
        Self::Object(
            pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        )
    }

    /// A string value.
    #[must_use]
    pub fn str(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// Look up a key on an object.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(m) => m.get(key),
            _ => None,
        }
    }

    /// The value as a string slice, if it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Serialise to JSON text.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => {
                out.push_str(if *b { "true" } else { "false" });
            }
            Self::Number(n) => {
                // JSON has one number type; printing an integral
                // value as `2` rather than `2.0` is what every other
                // encoder does, and the bound keeps the cast exact.
                #[allow(clippy::cast_possible_truncation)]
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    let _ = write!(out, "{}", *n as i64);
                } else {
                    let _ = write!(out, "{n}");
                }
            }
            Self::String(s) => write_string(s, out),
            Self::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Object(map) => {
                out.push('{');
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Control characters must be escaped or the output is not
            // valid JSON, and XML documents do contain them.
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse JSON text.
///
/// # Errors
///
/// Returns a description if the input is not valid JSON.
pub fn parse(input: &str) -> Result<Json, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut p = Parser {
        chars: &chars,
        pos: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.pos < chars.len() {
        return Err("trailing input after JSON value".to_owned());
    }
    Ok(v)
}

struct Parser<'a> {
    chars: &'a [char],
    pos: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while matches!(self.chars.get(self.pos), Some(' ' | '\t' | '\n' | '\r'))
        {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.chars.get(self.pos) {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::String(self.string()?)),
            Some('t') => self.literal("true", Json::Bool(true)),
            Some('f') => self.literal("false", Json::Bool(false)),
            Some('n') => self.literal("null", Json::Null),
            Some(_) => self.number(),
            None => Err("unexpected end of JSON".to_owned()),
        }
    }

    fn literal(&mut self, word: &str, v: Json) -> Result<Json, String> {
        if self.chars[self.pos..]
            .starts_with(word.chars().collect::<Vec<_>>().as_slice())
        {
            self.pos += word.len();
            Ok(v)
        } else {
            Err(format!("expected `{word}`"))
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        self.ws();
        if self.chars.get(self.pos) == Some(&'}') {
            self.pos += 1;
            return Ok(Json::Object(map));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            if self.chars.get(self.pos) != Some(&':') {
                return Err("expected `:` in object".to_owned());
            }
            self.pos += 1;
            let value = self.value()?;
            let _ = map.insert(key, value);
            self.ws();
            match self.chars.get(self.pos) {
                Some(',') => self.pos += 1,
                Some('}') => {
                    self.pos += 1;
                    return Ok(Json::Object(map));
                }
                _ => return Err("expected `,` or `}`".to_owned()),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut items = Vec::new();
        self.ws();
        if self.chars.get(self.pos) == Some(&']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.chars.get(self.pos) {
                Some(',') => self.pos += 1,
                Some(']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err("expected `,` or `]`".to_owned()),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.chars.get(self.pos) != Some(&'"') {
            return Err("expected a string".to_owned());
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            match self.chars.get(self.pos) {
                None => return Err("unterminated string".to_owned()),
                Some('"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.pos += 1;
                    let esc =
                        self.chars.get(self.pos).ok_or("trailing backslash")?;
                    self.pos += 1;
                    match esc {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let cp = self.hex4()?;
                            // A non-BMP character is escaped as a
                            // surrogate *pair*; neither half is a
                            // `char` on its own. Python's `json.dumps`
                            // escapes non-ASCII by default, so a client
                            // sending an emoji arrives this way.
                            let cp = if (0xD800..0xDC00).contains(&cp) {
                                if !matches!(
                                    self.chars.get(self.pos..self.pos + 2),
                                    Some(['\\', 'u'])
                                ) {
                                    return Err(
                                        "lone high surrogate".to_owned()
                                    );
                                }
                                self.pos += 2;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(
                                        "high surrogate not followed by a \
                                         low surrogate"
                                            .to_owned(),
                                    );
                                }
                                0x1_0000 + ((cp - 0xD800) << 10) + (lo - 0xDC00)
                            } else if (0xDC00..0xE000).contains(&cp) {
                                return Err("lone low surrogate".to_owned());
                            } else {
                                cp
                            };
                            out.push(
                                char::from_u32(cp).ok_or("bad code point")?,
                            );
                        }
                        c => out.push(*c),
                    }
                }
                Some(c) => {
                    out.push(*c);
                    self.pos += 1;
                }
            }
        }
    }

    /// Read exactly four hex digits of a `\\u` escape.
    fn hex4(&mut self) -> Result<u32, String> {
        let hex: String = self
            .chars
            .get(self.pos..self.pos + 4)
            .ok_or("truncated \\u escape")?
            .iter()
            .collect();
        self.pos += 4;
        u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape".to_owned())
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while matches!(
            self.chars.get(self.pos),
            Some(c) if c.is_ascii_digit()
                || matches!(c, '-' | '+' | '.' | 'e' | 'E')
        ) {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse()
            .map(Json::Number)
            .map_err(|_| format!("invalid number `{text}`"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) -> String {
        parse(text).expect("valid JSON").to_json()
    }

    #[test]
    fn scalars_round_trip() {
        assert_eq!(round_trip("null"), "null");
        assert_eq!(round_trip("true"), "true");
        assert_eq!(round_trip("false"), "false");
        assert_eq!(round_trip("\"hi\""), "\"hi\"");
    }

    #[test]
    fn integral_numbers_print_without_a_fraction() {
        // MCP ids are compared by clients; `2.0` where `2` was sent is
        // a different JSON value to a strict client.
        assert_eq!(round_trip("2"), "2");
        assert_eq!(round_trip("-17"), "-17");
        assert_eq!(round_trip("0"), "0");
        assert_eq!(round_trip("1.5"), "1.5");
        assert_eq!(round_trip("1e3"), "1000");
    }

    #[test]
    fn very_large_integral_numbers_do_not_wrap() {
        // Beyond 1e15 the i64 cast would be lossy, so the float path
        // must take over rather than silently truncating.
        let out = Json::Number(1e300).to_json();
        assert!(out.starts_with('1'), "{out}");
        assert!(parse(&out).is_ok(), "{out} did not re-parse");
    }

    #[test]
    fn objects_serialise_in_a_stable_order() {
        let text = round_trip(r#"{"b":1,"a":2}"#);
        assert_eq!(text, r#"{"a":2,"b":1}"#);
    }

    #[test]
    fn nesting_round_trips() {
        let src = r#"{"a":[1,[2,{"b":null}]],"c":{}}"#;
        assert_eq!(round_trip(src), src);
    }

    #[test]
    fn string_escapes_decode() {
        let v = parse(r#""a\nb\tc\"d\\e\/f\r\b\f""#).expect("valid");
        assert_eq!(v.as_str(), Some("a\nb\tc\"d\\e/f\r\u{8}\u{c}"));
    }

    #[test]
    fn control_characters_are_escaped_on_output() {
        // XML documents do contain these, and an unescaped control
        // character makes the whole response unparseable to the client.
        let out = Json::str("a\u{1}b\u{1f}").to_json();
        assert_eq!(out, r#""a\u0001b\u001f""#);
        assert_eq!(
            parse(&out).expect("re-parses").as_str(),
            Some("a\u{1}b\u{1f}")
        );
    }

    #[test]
    fn surrogate_pairs_decode_to_one_character() {
        // Python's `json.dumps` escapes non-ASCII by default, so a
        // client written against it sends emoji this way.
        assert_eq!(
            parse(r#""\ud83d\ude00""#).expect("valid").as_str(),
            Some("\u{1f600}")
        );
        assert_eq!(
            parse(r#""a\ud83d\ude00b""#).expect("valid").as_str(),
            Some("a\u{1f600}b")
        );
    }

    #[test]
    fn bmp_escapes_still_decode() {
        assert_eq!(
            parse(r#""\u00e9\u4e2d""#).expect("valid").as_str(),
            Some("é中")
        );
    }

    #[test]
    fn lone_surrogates_are_rejected() {
        // Rather than panicking or emitting an invalid character.
        assert!(parse(r#""\ud83d""#).is_err());
        assert!(parse(r#""\ude00""#).is_err());
        assert!(parse(r#""\ud83dx""#).is_err());
        assert!(parse(r#""\ud83d\u0041""#).is_err());
    }

    #[test]
    fn raw_utf8_passes_through_unescaped() {
        let src = "\"héllo 😀 中\"";
        assert_eq!(round_trip(src), src);
    }

    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        for bad in [
            "",
            "{",
            "[",
            "{\"a\"}",
            "{\"a\":}",
            "[1,",
            "\"unterminated",
            "tru",
            "nul",
            "\"\\",
            "\"\\u00\"",
            "\"\\uZZZZ\"",
            "@",
            "{\"a\":1}trailing",
            "1 2",
        ] {
            assert!(parse(bad).is_err(), "`{bad}` should not parse");
        }
    }

    #[test]
    fn whitespace_around_values_is_ignored() {
        assert_eq!(
            round_trip(" \n\t{ \"a\" : [ 1 , 2 ] }\r\n"),
            r#"{"a":[1,2]}"#
        );
    }

    #[test]
    fn get_and_as_str_are_type_safe() {
        let v = parse(r#"{"a":"x","b":1}"#).expect("valid");
        assert_eq!(v.get("a").and_then(Json::as_str), Some("x"));
        assert_eq!(v.get("b").and_then(Json::as_str), None);
        assert!(v.get("missing").is_none());
        assert!(Json::Null.get("a").is_none());
        assert!(Json::Null.as_str().is_none());
    }

    #[test]
    fn object_builder_matches_parsed_form() {
        let built =
            Json::object(vec![("a", Json::str("x")), ("b", Json::Number(1.0))]);
        assert_eq!(built.to_json(), r#"{"a":"x","b":1}"#);
    }

    #[test]
    fn deep_nesting_does_not_overflow_the_stack() {
        let depth = 200;
        let src = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse(&src).is_ok());
    }
}

#[cfg(test)]
mod escape_tests {
    use super::*;

    #[test]
    fn every_escape_survives_a_round_trip() {
        // Each of these has its own arm on both the reading and the
        // writing side, and a mismatch between the two corrupts the
        // document silently rather than failing.
        for s in [
            "back\\slash",
            "carriage\rreturn",
            "tab\there",
            "quote\"inside",
            "new\nline",
            "all\\ \" \n \r \t together",
        ] {
            let encoded = Json::str(s).to_json();
            assert_eq!(
                parse(&encoded).expect("re-parses").as_str(),
                Some(s),
                "round trip failed for {s:?} via {encoded}"
            );
        }
    }

    #[test]
    fn a_structural_error_names_what_was_expected() {
        // A bare "invalid JSON" would give a client nothing to act on.
        let e = parse(r#"{"a":1 "b":2}"#).expect_err("malformed");
        assert!(e.contains(',') || e.contains('}'), "{e}");

        let e = parse("[1 2]").expect_err("malformed");
        assert!(e.contains(',') || e.contains(']'), "{e}");
    }
}
