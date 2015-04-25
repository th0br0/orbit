extern crate core;

use std::convert::From;
use std::num::{ParseIntError, ParseFloatError};

#[derive(Clone, PartialEq, Debug)]
pub struct TLE {
    // line 1
    name: String,

    satellite_number: i16,
    classification: Classification,
    id_launch_year: i8,
    id_launch_number: i16,
    id_launch_piece: String,
    epoch_year: i16,
    epoch: f32,
    mean_motion_d: f32,
    mean_motion_dd: f32,
    bstar: f32,
    set_number: i32,

    // line 2
    inclination: f32,
    right_ascension: f32,
    eccentricity: f32,
    perigree: f32,
    mean_anomaly: f32,
    mean_motion: f32,
    revolution_number: i32
}

#[derive(Clone,PartialEq,Debug)]
pub enum Classification {
    Unclassified,
    Other
}

#[derive(Debug)]
pub enum DeserializationError {
    ParseError(String),
}

impl TLE {
    #[allow(dead_code)]
    fn validate() -> bool {
        panic!("Fix me. We need access to the raw line data.");
    }
}

impl From<core::num::ParseFloatError> for DeserializationError {
    fn from(err: core::num::ParseFloatError) -> DeserializationError {DeserializationError::ParseError(err.description().to_string())}
}

impl From<ParseIntError> for DeserializationError {
    fn from(err: ParseIntError) -> DeserializationError {DeserializationError::ParseError(err.description().to_string())}
}

fn fix_string(s: String) -> String {
    if s.starts_with("-.") { return s.replace("-.", "-0.") };

    match s.find("-") {
        Some(0) => return "-0.".to_string() + &s[1..(s.len() - 2)].to_string() + "e" + &s[(s.len() - 2)..s.len()],
        Some(_) => return "0.".to_string() + &s[0..(s.len() - 2)].to_string() + "e" + &s[(s.len() - 2)..s.len()],
        None => return format!("0.{}", s)
    }
}

pub fn deserialize_tle(input: String) -> Result<TLE, DeserializationError> {
    //TODO can we somehow enforce the parameter to have a fixed length?
    let lines : Vec<&str> = input.lines().collect();
    let name = &lines[0];
    let line1 = &lines[1];
    let line2 = &lines[2];

    let tle = TLE {
        name: name.to_string(),
        satellite_number: try!(line1[2..7].parse::<i16>()),
        classification: match line1.as_bytes()[8] {
            b'U' => Classification::Unclassified,
            _ => Classification::Other
        },

        id_launch_year: try!(line1[9..11].parse::<i8>()),
        id_launch_number: try!(line1[11..14].parse::<i16>()),
        id_launch_piece: line1[14..17].trim().to_string(),


        epoch_year: try!(line1[18..20].parse::<i16>()),
        epoch: try!(line1[20..32].trim().parse::<f32>()),
        mean_motion_d: try!(fix_string(line1[33..43].to_string()).parse::<f32>().map(|v| v * 2.0)),
        mean_motion_dd: try!(fix_string(line1[44..52].trim().to_string()).parse::<f32>().map(|v| v * 6.0)),
        bstar: try!(fix_string(line1[53..61].trim().to_string()).parse::<f32>()),
        set_number: try!(line1[64..68].trim().parse::<i32>()),

        // line 2
        inclination: try!(line2[08..16].trim().parse::<f32>()),
        right_ascension: try!(line2[17..25].trim().parse::<f32>()),
        eccentricity: try!(fix_string(line2[26..33].trim().to_string()).parse::<f32>()),
        perigree: try!(line2[34..42].trim().parse::<f32>()),
        mean_anomaly: try!(line2[43..51].trim().parse::<f32>()),
        mean_motion: try!(line2[52..63].trim().parse::<f32>()),
        revolution_number: try!(line2[63..68].parse::<i32>())
    };

    Ok(tle)
}

//FIXME is there some "not implemented" annotation we could use instead?"
#[allow(dead_code, unused_variables)]
pub fn serialize_tle(tle: TLE) -> String {
    panic!("Not implemented.");
}

#[cfg(test)]
mod test {
    pub const DATA : &'static str =
        "MOLNIYA 1-81\n\
        1 21426U 91043A   15108.55037587 -.00000207  00000-0 -31134-2 0  9992\n\
        2 21426  63.2890 290.2925 7228326 283.8438  15.2252  2.00627254174658";

    #[test]
    fn test_deserialize_tle() {
        let t = super::deserialize_tle(DATA.to_string());
        assert!(t.is_ok());
    }
}
