//! RGB color type and hex parsing.

use std::fmt;
use std::str::FromStr;

use crate::error::Error;

/// A 24-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// White (`#ffffff`).
    pub const WHITE: Rgb = Rgb::new(0xff, 0xff, 0xff);

    /// Creates a color from its channels.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Returns the color as `[r, g, b]`.
    pub const fn to_bytes(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl From<[u8; 3]> for Rgb {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::new(r, g, b)
    }
}

impl FromStr for Rgb {
    type Err = Error;

    /// Parses `#rrggbb` or `rrggbb`, case-insensitively.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || Error::InvalidColor(s.to_string());
        let trimmed = s.trim();
        let hex = trimmed.strip_prefix('#').unwrap_or(trimmed).as_bytes();
        if hex.len() != 6 {
            return Err(invalid());
        }
        let mut out = [0u8; 3];
        let (pairs, _) = hex.as_chunks::<2>();
        for (slot, [hi, lo]) in out.iter_mut().zip(pairs) {
            let hi = nibble(*hi).ok_or_else(invalid)?;
            let lo = nibble(*lo).ok_or_else(invalid)?;
            *slot = (hi << 4) | lo;
        }
        Ok(Self::from(out))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

fn nibble(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|d| d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_hash() {
        assert_eq!("#ff8800".parse(), Ok(Rgb::new(0xff, 0x88, 0x00)));
        assert_eq!("00ff7F".parse(), Ok(Rgb::new(0x00, 0xff, 0x7f)));
        assert_eq!("  #000000 ".parse(), Ok(Rgb::default()));
    }

    #[test]
    fn rejects_wrong_length() {
        for input in ["", "#", "#fff", "#ff88001", "ff880"] {
            let err = input.parse::<Rgb>().unwrap_err();
            assert_eq!(err, Error::InvalidColor(input.to_string()));
        }
    }

    #[test]
    fn rejects_non_hex_characters() {
        for input in ["#gg0000", "+12345", "#ff 800", "#ffé00"] {
            assert!(input.parse::<Rgb>().is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn display_round_trips() {
        let color = Rgb::new(0x12, 0xab, 0xef);
        assert_eq!(color.to_string(), "#12abef");
        assert_eq!(color.to_string().parse(), Ok(color));
    }

    #[test]
    fn byte_conversions() {
        assert_eq!(Rgb::from([1, 2, 3]).to_bytes(), [1, 2, 3]);
        assert_eq!(Rgb::WHITE.to_bytes(), [0xff; 3]);
    }
}
