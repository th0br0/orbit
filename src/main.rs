#![feature(core)]
#![feature(plugin)]
#![plugin(regex_macros)]
#![plugin(docopt_macros)]

extern crate rustc_serialize;
extern crate docopt;

mod tle;

use docopt::Docopt;
use std::fs::File;
use std::error::Error;

docopt!(Args derive Debug, "
Usage:  orbit [options] (--tle TLE) --satellite SATELLITE
        orbit (--help | --version)

Options:
    -s, --satellite        Satellite name to analyse
    --tle TLE              File containing TLE for parsing satellites
    -h, --help             Print this help message.
    -v, --version          Print version information.
");

fn main() {
    let args : Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());

    println!("{:?}", args);

    if !args.flag_tle.is_empty(){
        let mut file = match File::open(&args.flag_tle) {
            Err(why) => panic!("couldn't open {}: {}", args.flag_tle, Error::description(&why)),
            Ok(file) => file
        };

        let tles = tle::parse_file(&mut file);

        for tle in tles {
            println!("{:?}", tle);
        }
    }
}
