use tle;
use satellite;
use satellite::Satellite;

use std::cmp::Ordering::Equal;
use chrono::*;
use std::fs::File;
use std::ops::*;
use std::fmt;
use std::io::prelude::*;

use sdl2;
use sdl2::render::Renderer;
use sdl2::pixels::Color;
use sdl2::keyboard::Keycode;
use sdl2::rect::{Rect, Point};
use sdl2_gfx::primitives::*;

#[derive(Debug)]
struct Sample {
    timestamp: DateTime<UTC>,
    angle: f64,
    radius: f64,
}

impl fmt::Display for Sample {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{} {:.6} {:.6}",
               self.timestamp.format("%H:%M:%S"),
               self.angle,
               self.radius)
    }
}

pub fn calculate(tle: tle::TLE,
                 start: DateTime<UTC>,
                 end: DateTime<UTC>,
                 stepping: Duration,
                 flag_visualize: bool,
                 output: Option<File>) {
    let satellite: tle::Satellite = satellite::Satellite::new(satellite::EARTH, tle);

    let steps = 1 + (end.sub(start).num_seconds() / stepping.num_seconds()) as i32;

    let a = satellite.semimajor_axis_approx();
    println!("Total number samples: {}", steps);
    println!("Semi-major axis: {:?}km", a.unwrap());

    let samples: Vec<Sample> = (0..steps)
                                   .map(|i| {
                                       let time = start + (stepping * i);
                                       let e = satellite.eccentric_anomaly(time).unwrap();
                                       let v = (satellite.true_anomaly(e) + 360_f64) % 360_f64;

                                       let r_v = a.map(|a| {
                                                      a *
                                                      ((1.0 - satellite.eccentricity().powi(2)) /
                                                       (1.0 + satellite.eccentricity() * v.to_radians().cos()))
                                                  })
                                                  .unwrap();

                                       Sample {
                                           timestamp: time,
                                           angle: v,
                                           radius: r_v,
                                       }
                                   })
                                   .collect();

    if let Some(mut file) = output {
        let result: Vec<String> = samples.iter().map(|s| format!("{}", s)).collect();
        let _ = file.write_all(result.join("\n").as_bytes());
    }

    if flag_visualize {
        visualize(a.unwrap(),
                  satellite.distance_apogee_approx().unwrap(),
                  satellite.distance_perigee_approx().unwrap(),
                  samples);
    }
}

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
