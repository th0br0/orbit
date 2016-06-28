use tle;
use satellite;
use satellite::Satellite;
use body::EARTH;

use std::cmp::Ordering::Equal;
use chrono::*;
use std::fs::File;
use std::ops::*;
use std::fmt;
use std::io::prelude::*;
use std::f64::consts::PI;

use sdl2;
use sdl2::render::Renderer;
use sdl2::pixels::Color;
use sdl2::keyboard::Keycode;
use sdl2::rect::{Rect, Point};
use sdl2_gfx::primitives::*;

#[derive(Debug)]
struct Sample {
    timestamp: DateTime<UTC>,

    real_anomaly: f64,
    radius: f64,
    distance: f64,
    longitude_ascending_node: f64,
    argument_periapsis: f64,

    lambda_g: f64,

    theta: f64,
    lambda: f64,

    azimuth: f64,
    elevation: f64,
}

impl fmt::Display for Sample {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
               self.timestamp.format("%H:%M:%S"),
               self.real_anomaly,
               self.radius,
               self.distance,
               self.longitude_ascending_node,
               self.argument_periapsis,
               self.lambda_g,
               self.theta,
               self.lambda,
               self.azimuth,
               self.elevation)
    }
}

pub fn calculate(tle: tle::TLE,
                 start: DateTime<UTC>,
                 end: DateTime<UTC>,
                 stepping: Duration,
                 flag_visualize: bool,
                 output: Option<File>,
                 latitude: f64,
                 longitude: f64,
                 radius: f64) {
    let satellite: tle::Satellite = satellite::Satellite::new(EARTH, tle);

    let steps = 1 + (end.sub(start).num_seconds() / stepping.num_seconds()) as i32;

    let a = satellite.semimajor_axis_approx();
    println!("Total number samples: {}", steps);
    println!("Semi-major axis: {:?}km", a.unwrap());

    let theta_ground = latitude.to_radians();
    let lambda_ground = longitude.to_radians();
    let radius_ground = radius;

    println!("Ground station: R: {}, Lat {}, Lat Rad {}, Lon {}, Lon Rad {}",
             radius_ground,
             latitude,
             theta_ground,
             longitude,
             lambda_ground);

    let samples: Vec<Sample> = (0..steps)
                                   .map(|i| {
                                       let time = start + (stepping * i);

                                       let delta_t = time.sub(satellite.timestamp());

                                       let start_epoch = (satellite.timestamp().sub(UTC.yo(start.year(), 1)
                                                                   .and_time(NaiveTime::from_num_seconds_from_midnight(0,0)).unwrap()).num_nanoseconds()
                                           .unwrap() as f64) * 1.0e-9 /86400f64;

                                       let delta_t_epoch = (delta_t.num_nanoseconds().unwrap() as f64) * 1.0e-9 / 86400f64;

                                       let e = satellite.eccentric_anomaly(time).unwrap();
                                       let v = satellite.true_anomaly(e);

                                       let r_v = a.map(|a| {
                                           a *
                                               ((1.0 - satellite.eccentricity().powi(2)) /
                                                (1.0 + satellite.eccentricity() * v.cos()))
                                       }).unwrap();


                                       let omega_big = satellite.longitude_ascending_node(time).unwrap();
                                       let omega_small = satellite.argument_periapsis(time).unwrap();

                                       let lambda_g = (satellite.body().lambda + (start_epoch + delta_t_epoch) * satellite.body().we) % 360.0;

                                       let i_rad = satellite.inclination().to_radians();
                                       let theta = ((omega_small + v).sin() * i_rad.sin()).asin();
                                       let l1 = (theta.tan() / i_rad.tan()).atan2(
                                                    (omega_small + v).cos() / theta.cos()
                                               );

                                       let lambda = (l1 + omega_big - lambda_g.to_radians());
                                       let lambda_tmp = lambda.to_degrees();

                                       // XXX find a better way to do this.
                                       let lambda_deg : f64 = if (lambda_tmp < -180.0) {
                                              lambda_tmp % 180.0 
                                           } else if (lambda_tmp > 180.0) {
                                               -180.0 + (lambda_tmp % 180.0)
                                           } else { lambda_tmp };


                                       let beta = (theta_ground.sin() * theta.sin() + theta_ground.cos() * theta.cos() * (lambda - lambda_ground).cos()).acos();
                                       let distance = (radius_ground.powi(2) + r_v.powi(2) - 2.0*radius_ground*r_v*beta.cos()).sqrt();

                                       let elevation = ((r_v.powi(2) - distance.powi(2) - radius_ground.powi(2))/(2.0 * radius_ground * distance)).asin();

                                       let alpha_sin = ((lambda - lambda_ground).sin() * (0.5 * PI - theta).sin()) / beta.sin();
                                       let alpha_cos = ((0.5 * PI - theta).cos() - (0.5 * PI - theta_ground).cos() * beta.cos()) / ((0.5 * PI - theta_ground).sin() * beta.sin());
                                       let azimuth = alpha_sin.atan2(alpha_cos);

                                       Sample {
                                           timestamp: time,

                                           real_anomaly: (v.to_degrees() + 360f64) % 360f64,
                                           radius: r_v,
                                           distance: distance,

                                           longitude_ascending_node: omega_big.to_degrees(),
                                           argument_periapsis: omega_small.to_degrees(),
                                           lambda_g: lambda_g,
                                           theta: theta.to_degrees(),
                                           lambda: ((PI + lambda) % PI).to_degrees(),
                                           azimuth: azimuth.to_degrees(),
                                           elevation: elevation.to_degrees()
                                       }
                                   })
        .filter(|s| s.elevation > -3.0)
                                   .collect();

    if let Some(mut file) = output {
        let result: Vec<String> = samples.iter().map(|s| format!("{}", s)).collect();
        let _ = file.write_all(result.join("\n").as_bytes());
    }

    //    if flag_visualize {
    // visualize(a.unwrap(),
    // satellite.distance_apogee_approx().unwrap(),
    // satellite.distance_perigee_approx().unwrap(),
    // samples);
    // }
}
