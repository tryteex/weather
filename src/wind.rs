//! Module responsible for detect wind direction.
//!

/// Wind Direction and Degrees
#[derive(Debug)]
pub enum WindDeg {
    None,
    Unknown,
    North,
    NorthNorthEast,
    NorthEast,
    EastNorthEast,
    East,
    EastSouthEast,
    SouthEast,
    SouthSouthEast,
    South,
    SouthSouthWest,
    SouthWest,
    WestSouthWest,
    West,
    WestNorthWest,
    NorthWest,
    NorthNorthWest,
}

impl WindDeg {
    /// Get wind direction from degrees
    pub fn get(degree: Option<u16>) -> WindDeg {
        let degree = match degree {
            Some(degree) => degree,
            None => return WindDeg::None,
        };
        match degree {
            0..=11 | 349..=360 => WindDeg::North,
            12..=33 => WindDeg::NorthNorthEast,
            34..=56 => WindDeg::NorthEast,
            57..=78 => WindDeg::EastNorthEast,
            79..=101 => WindDeg::East,
            102..=123 => WindDeg::EastSouthEast,
            124..=146 => WindDeg::SouthEast,
            147..=168 => WindDeg::SouthSouthEast,
            169..=191 => WindDeg::South,
            192..=213 => WindDeg::SouthSouthWest,
            214..=236 => WindDeg::SouthWest,
            237..=258 => WindDeg::WestSouthWest,
            259..=281 => WindDeg::West,
            282..=303 => WindDeg::WestNorthWest,
            304..=326 => WindDeg::NorthWest,
            327..=348 => WindDeg::NorthNorthWest,
            _ => WindDeg::Unknown,
        }
    }
}
