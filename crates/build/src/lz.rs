//! Bounded reader for lz-string "encoded URI component" text.
//!
//! `lz_str` decompresses without an output limit, so a short hostile code can expand to
//! gigabytes; this reader stops as soon as the output passes the caller's limit.

const BITS_PER_SYMBOL: u32 = 6;
const FIRST_ENTRY: usize = 3;
const MAX_WIDTH: u32 = 30;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    TooLarge,
}

struct Bits<'a> {
    symbols: &'a [u8],
    index: usize,
    value: u8,
    mask: u8,
}

impl Bits<'_> {
    fn read(&mut self, count: u32) -> Result<u32, Error> {
        let mut bits = 0;
        for power in 0..count {
            if self.mask == 0 {
                let symbol = *self.symbols.get(self.index).ok_or(Error::Invalid)?;
                self.value = symbol_value(symbol)?;
                self.index += 1;
                self.mask = 1 << (BITS_PER_SYMBOL - 1);
            }
            if self.value & self.mask != 0 {
                bits |= 1 << power;
            }
            self.mask >>= 1;
        }
        Ok(bits)
    }
}

fn symbol_value(symbol: u8) -> Result<u8, Error> {
    Ok(match symbol {
        b'A'..=b'Z' => symbol - b'A',
        b'a'..=b'z' => symbol - b'a' + 26,
        b'0'..=b'9' => symbol - b'0' + 52,
        // Form posts turn `+` into a space; lz-string accepts both.
        b'+' | b' ' => 62,
        b'-' => 63,
        // The padding symbol carries no bits.
        b'$' => 0,
        _ => return Err(Error::Invalid),
    })
}

/// Decompresses `code` into UTF-16 units, refusing output longer than `limit` units.
pub fn decompress_uri(code: &str, limit: usize) -> Result<Vec<u16>, Error> {
    let mut bits = Bits {
        symbols: code.as_bytes(),
        index: 0,
        value: 0,
        mask: 0,
    };
    let first = match bits.read(2)? {
        0 => bits.read(8)?,
        1 => bits.read(16)?,
        2 => return Ok(Vec::new()),
        _ => return Err(Error::Invalid),
    };
    if limit == 0 {
        return Err(Error::TooLarge);
    }
    let mut output = vec![first as u16];
    // Every dictionary entry is text already emitted, so it is kept as a span of `output`.
    let mut entries: Vec<(usize, usize)> = vec![(0, 1)];
    let mut previous = (0, 1);
    let mut enlarge_in = 4u32;
    let mut width = 3u32;
    loop {
        if width > MAX_WIDTH {
            return Err(Error::Invalid);
        }
        let mut code = bits.read(width)? as usize;
        let literal = match code {
            0 => Some(bits.read(8)? as u16),
            1 => Some(bits.read(16)? as u16),
            2 => return Ok(output),
            _ => None,
        };
        if literal.is_some() {
            entries.push((output.len(), 1));
            code = entries.len() + FIRST_ENTRY - 1;
            enlarge_in -= 1;
        }
        if enlarge_in == 0 {
            enlarge_in = 1 << width;
            width += 1;
        }
        let start = output.len();
        let (previous_start, previous_len) = previous;
        let known = entries.get(code - FIRST_ENTRY).copied();
        let len = match (literal, known) {
            (Some(_), _) => 1,
            (None, Some((_, len))) => len,
            (None, None) if code - FIRST_ENTRY == entries.len() => previous_len + 1,
            (None, None) => return Err(Error::Invalid),
        };
        if start + len > limit {
            return Err(Error::TooLarge);
        }
        match (literal, known) {
            (Some(unit), _) => output.push(unit),
            (None, Some((from, len))) => output.extend_from_within(from..from + len),
            (None, None) => {
                output.extend_from_within(previous_start..previous_start + previous_len);
                output.push(output[previous_start]);
            }
        }
        // `previous` plus the first unit of this entry, which sits right after it.
        entries.push((previous_start, previous_len + 1));
        enlarge_in -= 1;
        previous = (start, len);
        if enlarge_in == 0 {
            enlarge_in = 1 << width;
            width += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compress(text: &str) -> String {
        lz_str::compress_to_encoded_uri_component(text)
    }

    #[test]
    fn output_matches_lz_str_for_varied_text_and_real_share_codes() {
        let mut varied = String::from("{\"v\":3,\"n\":\"Żółw 🐢\"}");
        for index in 0..4_000u32 {
            varied.push_str(&(index.wrapping_mul(2_654_435_761) % 9_973).to_string());
            varied.push(char::from_u32(0x100 + index % 700).unwrap());
        }
        let texts = [
            "a".to_owned(),
            "abababababababab".to_owned(),
            "x".repeat(10_000),
            varied,
        ];
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../engine/tests/fixtures/native-share-codes.json"
        ))
        .unwrap();
        let codes = texts.iter().map(|text| compress(text)).chain(
            fixtures
                .as_array()
                .unwrap()
                .iter()
                .map(|fixture| fixture["code"].as_str().unwrap().to_owned()),
        );

        for code in codes {
            let expected = lz_str::decompress_from_encoded_uri_component(&code).unwrap();
            assert_eq!(decompress_uri(&code, usize::MAX), Ok(expected));
        }
    }

    #[test]
    fn expansion_past_the_limit_stops_early() {
        let bomb = compress(&"a".repeat(2_000_000));

        assert!(bomb.len() < 5_000);
        assert_eq!(decompress_uri(&bomb, 200_000), Err(Error::TooLarge));
        assert_eq!(decompress_uri(&compress("abc"), 2), Err(Error::TooLarge));
        assert_eq!(
            decompress_uri(&compress("abc"), 3),
            Ok("abc".encode_utf16().collect())
        );
    }

    #[test]
    fn malformed_codes_are_invalid() {
        let code = compress("a build code that gets cut short");

        for broken in ["", "!", "not a build", &code[..code.len() / 2]] {
            assert_eq!(
                decompress_uri(broken, usize::MAX),
                Err(Error::Invalid),
                "{broken}"
            );
        }
        assert_eq!(decompress_uri(&compress(""), usize::MAX), Ok(Vec::new()));
    }
}
