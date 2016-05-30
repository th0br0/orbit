extern crate core;

use std::convert::From;
use std::num::ParseIntError;
use std::fs::File;
use std::error::Error;
use std::io::Read;
use chrono::*;
use satellite;

#[derive(Clone, PartialEq, Debug)]
pub struct TLE {
    // line 1
    pub name: String,

    pub satellite_number: i16,
    pub classification: Classification,
    pub id_launch_year: i8,
    pub id_launch_number: i16,
    pub id_launch_piece: String,
    pub timestamp: DateTime<UTC>,
    pub mean_motion_d: f64,
    pub mean_motion_dd: f64,
    pub bstar: f64,
    pub set_number: i32,

    // line 2
    pub inclination: f64,
    pub right_ascension: f64,
    pub eccentricity: f64,
    pub perigree: f64,
    pub mean_anomaly: f64,
    pub mean_motion: f64,
    pub revolution_number: i32,

    valid: bool,
}

#[derive(Clone,PartialEq,Debug)]
pub enum Classification {
    Unclassified,
    Other,
}

#[derive(Debug)]
pub enum DeserializationError {
    ParseError(String),
}

impl From<core::num::ParseFloatError> for DeserializationError {
    fn from(err: core::num::ParseFloatError) -> DeserializationError {
        DeserializationError::ParseError(err.description().to_string())
    }
}

impl From<ParseIntError> for DeserializationError {
    fn from(err: ParseIntError) -> DeserializationError {
        DeserializationError::ParseError(err.description().to_string())
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Satellite {
    body: satellite::Body,
    tle: TLE,
}

impl satellite::Satellite<TLE> for Satellite {
    fn new(body: satellite::Body, tle: TLE) -> Satellite {
        Satellite {
            body: body,
            tle: tle,
        }
    }
    fn body(&self) -> &satellite::Body {
        &self.body
    }

    fn mean_anomaly(&self) -> f64 {
        self.tle.mean_anomaly as f64
    }

    fn mean_motion_d(&self) -> f64 {
        self.tle.mean_motion_d as f64
    }

    fn mean_motion(&self) -> f64 {
        self.tle.mean_motion as f64
    }

    fn eccentricity(&self) -> f64 {
        self.tle.eccentricity as f64
    }

    fn inclination(&self) -> f64 {
        self.tle.inclination as f64
    }

    fn timestamp(&self) -> DateTime<UTC> {
        self.tle.timestamp
    }
}

// FIXME - this function is scary
fn fix_string(s: String) -> String {
    if s.starts_with("-.") {
        return s.replace("-.", "-0.");
    };

    if s.starts_with(".") {
        return "0".to_string() + &s;
    }

    match s.find("-") {
        Some(0) => {
            return "-0.".to_string() + &s[1..(s.len() - 2)].to_string() + "e" +
                   &s[(s.len() - 2)..s.len()]
        }
        Some(_) => {
            return "0.".to_string() + &s[0..(s.len() - 2)].to_string() + "e" +
                   &s[(s.len() - 2)..s.len()]
        }
        None => return format!("0.{}", s).replace("+0", "e+0"),
    }
}

fn line_checksum(line: String) -> bool {
    let calculated_checksum = line.chars()
                                  .rev()
                                  .skip(1)
                                  .filter(|&c| c.is_digit(10) || c == "-".chars().next().unwrap())
                                  .fold(0, |acc, c| acc + c.to_digit(10).unwrap_or(1)) %
                              10;

    let expected_checksum = line.chars().rev().next().and_then(|c| c.to_digit(10)).unwrap();

    calculated_checksum == expected_checksum
}

impl TLE {
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn new(input: &String) -> Result<TLE, DeserializationError> {
        // TODO can we somehow enforce the parameter to have a fixed length?
        let lines: Vec<&str> = input.lines().collect();
        let name = &lines[0];
        let line1 = &lines[1];
        let line2 = &lines[2];

        let tle = TLE {
            name: name.to_string(),
            satellite_number: try!(line1[2..7].parse::<i16>()),
            classification: match line1.as_bytes()[8] {
                b'U' => Classification::Unclassified,
                _ => Classification::Other,
            },

            id_launch_year: try!(line1[9..11].parse::<i8>()),
            id_launch_number: try!(line1[11..14].parse::<i16>()),
            id_launch_piece: line1[14..17].trim().to_string(),

            timestamp: try!({
                let year = 2000 + line1[18..20].parse::<i32>().unwrap();
                let days = line1[20..23].parse::<u32>().unwrap();
                let offset = 3600_f64 * 24_f64 * line1[23..32].parse::<f64>().unwrap();
                UTC.yo(year, days)
                   .and_time(NaiveTime::from_num_seconds_from_midnight(offset as u32, ((offset % 1.0)* 1.0e9) as u32))
                   .ok_or(DeserializationError::ParseError("Timestamp parsing failed.".to_string()))
            }),
            mean_motion_d: try!(fix_string(line1[33..43].trim().to_string())
                                    .parse::<f64>()
                                    .map(|v| v * 2.0)),
            mean_motion_dd: try!(fix_string(line1[44..52].trim().to_string())
                                     .parse::<f64>()
                                     .map(|v| v * 6.0)),
            bstar: try!(fix_string(line1[53..61].trim().to_string()).parse::<f64>()),
            set_number: try!(line1[64..68].trim().parse::<i32>()),

            // line 2
            inclination: try!(line2[08..16].trim().parse::<f64>()),
            right_ascension: try!(line2[17..25].trim().parse::<f64>()),
            eccentricity: try!(fix_string(line2[26..33].trim().to_string()).parse::<f64>()),
            perigree: try!(line2[34..42].trim().parse::<f64>()),
            mean_anomaly: try!(line2[43..51].trim().parse::<f64>()),
            mean_motion: try!(line2[52..63].trim().parse::<f64>()),
            revolution_number: try!(line2[63..68].trim().parse::<i32>()),

            valid: line_checksum(line1.to_string()) && line_checksum(line2.to_string()),
        };

        // Maybe return Err if is_valid = false?
        Ok(tle)
    }

    pub fn serialize(&self) -> String {
        panic!("IMPLEMENT ME!");
    }
}

pub fn parse_file(file: &mut File) -> Vec<TLE> {
    // maybe move the IO interaction into the caller?
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("couldn't read TLE file: {}", Error::description(&why)),
        Ok(_) => {}
    }

    let lines = s.lines().collect::<Vec<&str>>();
    let chunks: Vec<String> = lines.chunks(3)
                                   .map(|chunk| {
                                       chunk.iter()
                                            .fold(String::new(), |acc, c| acc + c.trim() + "\n")
                                   })
                                   .collect();

    // FIXME - we're dropping invalid TLEs silently!!!
    //

    chunks.iter()
          .map(|c| TLE::new(c))
          .filter(|t| t.is_ok())
          .map(|t| t.unwrap())
          .collect()
}

#[cfg(test)]
pub mod test {
    pub const DATA: &'static str = "MOLNIYA 1-81\n1 21426U 91043A   15108.55037587 -.00000207  \
                                    00000-0 -31134-2 0  9992\n2 21426  63.2890 290.2925 7228326 \
                                    283.8438  15.2252  2.00627254174658";

    #[test]
    fn test_deserialize_tle() {
        let t = super::TLE::new(&DATA.to_string());
        assert!(t.is_ok() && t.unwrap().is_valid());
    }

    #[test]
    fn test_fix_string() {
        // XXX implement me
    }

    #[test]
    fn test_serialize_tle() {
        let t = super::TLE::new(&DATA.to_string()).unwrap();
        // t.serialize();
    }
}
