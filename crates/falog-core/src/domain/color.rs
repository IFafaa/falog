use crate::Error;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use std::fmt;
use std::str::FromStr;

/// An sRGB color, stored and exchanged as `#rrggbb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b])
    }

    pub const fn from_u32(hex: u32) -> Self {
        Self([(hex >> 16) as u8, (hex >> 8) as u8, hex as u8])
    }

    pub fn to_hex(self) -> String {
        let [r, g, b] = self.0;
        format!("#{r:02x}{g:02x}{b:02x}")
    }
}

/// Default company colors: the player colors of Zed's One Dark theme.
pub const PALETTE: [Rgb; 8] = [
    Rgb::from_u32(0x74ade8),
    Rgb::from_u32(0xbf956a),
    Rgb::from_u32(0xa1c181),
    Rgb::from_u32(0xb477cf),
    Rgb::from_u32(0x6eb4bf),
    Rgb::from_u32(0xd07277),
    Rgb::from_u32(0xdec184),
    Rgb::from_u32(0xbe5046),
];

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl FromStr for Rgb {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.trim().trim_start_matches('#');
        let invalid = || Error::invalid(format!("invalid color \"{s}\", expected #rrggbb"));
        if hex.len() != 6 {
            return Err(invalid());
        }
        u32::from_str_radix(hex, 16)
            .map(Self::from_u32)
            .map_err(|_| invalid())
    }
}

impl ToSql for Rgb {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.to_hex().into())
    }
}

impl FromSql for Rgb {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()?
            .parse()
            .map_err(|e: Error| FromSqlError::Other(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let color: Rgb = "#74ADE8".parse().unwrap();
        assert_eq!(color, Rgb::new(0x74, 0xad, 0xe8));
        assert_eq!(color.to_hex(), "#74ade8");
        assert_eq!("74ade8".parse::<Rgb>().unwrap(), color);
    }

    #[test]
    fn rejects_malformed_colors() {
        assert!("#fff".parse::<Rgb>().is_err());
        assert!("#gggggg".parse::<Rgb>().is_err());
    }
}
