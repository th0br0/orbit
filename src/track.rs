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
    longitude_ascending_node: f64,
    argument_periapsis: f64,

    lambda_g: f64,

    theta: f64,
    lambda: f64
}

impl fmt::Display for Sample {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{} {:.6} {:.6} {:.4} {:.4} {:.4} {:.4} {:.4}",
               self.timestamp.format("%H:%M:%S"),
               self.real_anomaly,
               self.radius,
               self.longitude_ascending_node,
               self.argument_periapsis,
               self.lambda_g,
               self.theta,
               self.lambda)
    }
}

pub fn calculate(tle: tle::TLE,
                 start: DateTime<UTC>,
                 end: DateTime<UTC>,
                 stepping: Duration,
                 flag_visualize: bool,
                 output: Option<File>) {
    let satellite: tle::Satellite = satellite::Satellite::new(EARTH, tle);

    let steps = 1 + (end.sub(start).num_seconds() / stepping.num_seconds()) as i32;

    let a = satellite.semimajor_axis_approx();
    println!("Total number samples: {}", steps);
    println!("Semi-major axis: {:?}km", a.unwrap());

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
                                       let lambda = (l1 + omega_big - lambda_g.to_radians()).to_degrees();

                                       // XXX find a better way to do this.
                                       let lambda_normalized : f64 = if (lambda < -180.0) {
                                              lambda % 180.0 
                                           } else if (lambda > 180.0) {
                                               -180.0 + (lambda % 180.0)
                                           } else { lambda };

                                       Sample {
                                           timestamp: time,

                                           real_anomaly: (v.to_degrees() + 360f64) % 360f64,
                                           radius: r_v,
                                           longitude_ascending_node: omega_big.to_degrees(),
                                           argument_periapsis: omega_small.to_degrees(),
                                           lambda_g: lambda_g,
                                           theta: theta.to_degrees(),
                                           lambda: lambda_normalized
                                       }
                                   })
                                   .collect();

    if let Some(mut file) = output {
        let result: Vec<String> = samples.iter().map(|s| format!("{}", s)).collect();
        let _ = file.write_all(result.join("\n").as_bytes());
    }

/*    if flag_visualize {
        visualize(a.unwrap(),
                  satellite.distance_apogee_approx().unwrap(),
                  satellite.distance_perigee_approx().unwrap(),
                  samples);
    }*/
}
/*
fn visualize(a: f64, r_apogee: f64, r_perigee: f64, mut samples: Vec<Sample>) {
    // normalize the radii
    // determine maximum radius.
    samples.sort_by(|a, b| a.radius.abs().partial_cmp(&b.radius.abs()).unwrap_or(Equal));

    let radius_max = samples.last().unwrap().radius;

    let mut samples_normalized: Vec<Sample> = samples.iter()
                                                     .map(|s| {
                                                         Sample {
                                                             timestamp: s.timestamp,
                                                             angle: s.angle,
                                                             radius: s.radius / radius_max,
                                                         }
                                                     })
                                                     .collect();
    samples_normalized.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    draw_sdl(r_apogee, r_perigee, &samples_normalized);
}

fn draw_sdl(r_apogee: f64, r_perigee: f64, samples: &Vec<Sample>) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();

    let window = video_subsys.window("orbit movement", 1024, 1024)
                             .position_centered()
                             .opengl()
                             .build()
                             .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.clear();
    draw(&mut renderer, r_apogee, r_perigee, samples);

    renderer.present();


    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            use sdl2::event::Event;

            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                }
                _ => {}
            }
        }
        // The rest of the game loop goes here...
    }
}

fn draw(renderer: &mut sdl2::render::Renderer,
        r_apogee: f64,
        r_perigee: f64,
        samples: &Vec<Sample>) {
    let viewport_orig = renderer.viewport();

    let scale = 0.9f32;
    let mut viewport = Rect::from_center(viewport_orig.center(),
                                     ((viewport_orig.width() as f32) * scale) as u32,
                                     ((viewport_orig.height() as f32) * scale) as u32);

    //viewport.offset(((1_f64 - scale) as u32 * viewport_orig.width()) as i32, ((1_f64 - scale) as u32* viewport_orig.height()) as i32);
    //renderer.set_viewport(Some(viewport));

    let w = viewport.width() as f64;
    let h = viewport.height() as f64;
    let cx = viewport.center().x() as i16;
    let cy = viewport.center().y() as i16;

    println!("Apogee {} Perigee {}", r_apogee, r_perigee);
    let r_apogee_n = r_apogee / (r_apogee + r_perigee);
    let r_perigee_n = r_perigee / (r_apogee + r_perigee);

    let r_apogee_l = (r_apogee_n * w);
    let r_perigee_l = (r_perigee_n * w);

    let planet_r: i16 = 32;
    let satellite_r: i16 = 4;

    let planet_color = Color::RGB(0, 0, 255);
    let apogee_color = Color::RGB(255, 0, 0);
    let perigee_color = Color::RGB(0, 255, 0);
    let satellite_color = Color::RGB(192, 192, 192);

    // Draw planet
    let planet_cx = r_apogee_l as i16;
    let planet_cy = cy;

    // Draw apogee & perigee
    let _ = renderer.hline(viewport.left() as i16, viewport.left() as i16 + planet_cx, planet_cy, apogee_color);
    let _ = renderer.hline(viewport.left() as i16 + planet_cx, viewport.right() as i16, planet_cy, perigee_color);

    let _ = renderer.filled_circle(planet_cx, planet_cy, planet_r, planet_color);
    let _ = renderer.pixel(planet_cx, planet_cy, satellite_color);

    let _ = renderer.pixel(cx, cy, satellite_color);


    let draw_satellite = |s: &Sample, c: Color| {
        let x = (s.angle.to_radians().cos() * s.radius * r_apogee_l) as i16;
        let y = (s.angle.to_radians().sin() * s.radius * r_apogee_l) as i16;

        renderer.filled_circle(viewport.left() as i16 + planet_cx + x, planet_cy + y, satellite_r, c);
    };

    for sample in samples {
        draw_satellite(sample, satellite_color);
    }

    draw_satellite(samples.first().unwrap(), perigee_color);
    draw_satellite(samples.last().unwrap(), apogee_color);
}
*/
