#![feature(plugin)]
#![plugin(regex_macros)]
#![plugin(docopt_macros)]

extern crate rustc_serialize;
extern crate docopt;
extern crate roots;
extern crate chrono;
extern crate sdl2;
extern crate sdl2_gfx;

mod tle;
mod satellite;
mod movement;

use docopt::Docopt;
use std::fs::File;
use std::error::Error;
use chrono::*;

docopt!(Args derive Debug, "
Usage:
  orbit movement [options] --tle TLE --satellite SATELLITE --start START --end END --stepping STEPPING
  orbit track [options] --tle TLE --satellite SATELLITE --start START --end END --stepping STEPPING
  orbit -h | --help
  orbit -V | --version

Options:
    -s, --satellite        Satellite name to analyse.
    --tle TLE              File containing TLE for parsing satellites.
    -o OUT, --output OUT   Write output to file.
    --start START          Start timestamp.
    --end END              End timestamp.
    --stepping STEPPING    Time stepping in [s]. [default=300]
    -v, --visualize        Visualise computed data.
    -h, --help             Print this help message.
    -V, --version          Print version information.
");

fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());

    let stepping = Duration::seconds(args.flag_stepping.parse::<i64>().ok().unwrap_or(300));
    let start = args.flag_start.parse::<DateTime<UTC>>().unwrap();
    let end = args.flag_end.parse::<DateTime<UTC>>().unwrap();


    // For now, docopts ensures that we've got a tle parameter
    let mut file = match File::open(&args.flag_tle) {
        Err(why) => {
            panic!("Couldn't open {}: {}",
                   args.flag_tle,
                   Error::description(&why))
        }
        Ok(file) => file,
    };

    let tles = tle::parse_file(&mut file);

    let satellite = match tles.iter().find(|t| args.arg_SATELLITE == t.name) {
        Some(satellite) => satellite.clone(),
        None => {
            panic!("Couldn't find satellite '{}' in tle input.",
                   args.arg_SATELLITE)
        }
    };

    if args.cmd_movement {
        movement::calculate(satellite,
                            start,
                            end,
                            stepping,
                            args.flag_visualize,
                            File::create(&args.flag_output).ok());
    } else if args.cmd_track {
        println!("{:?}", args);
    } 
}
