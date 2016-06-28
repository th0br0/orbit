use std::f64::consts::PI;
use chrono::*;
use std::ops::Sub;
use roots;
use std::cell::Cell;
use body::Body;

pub trait Satellite<T> {
    fn new(body: Body, t: T) -> Self;
    fn body(&self) -> &Body;
    fn right_ascension(&self) -> f64;
    fn perigree(&self) -> f64;
    fn mean_motion(&self) -> f64;
    fn mean_motion_d(&self) -> f64;
    fn mean_anomaly(&self) -> f64;
    fn eccentricity(&self) -> f64;
    fn inclination(&self) -> f64;
    fn timestamp(&self) -> DateTime<UTC>;

    fn period_of_revolution(&self) -> f64 {
        86400_f64 / self.mean_motion()
    }

    fn semimajor_axis_ideal(&self) -> f64 {
        (self.body().mu * (self.period_of_revolution() / (2f64 * PI)).powi(2)).cbrt()
    }

    fn semiminor_axis_ideal(&self) -> f64 {
        let semimajor = self.semimajor_axis_ideal();
        let inner = semimajor.powi(2) - (self.eccentricity() * semimajor).powi(2);

        inner.sqrt()
    }

    fn distance_apogee_approx(&self) -> Option<f64> {
        self.semimajor_axis_approx().map(|a| a * (1.0 + self.eccentricity()))
    }

    fn distance_perigee_approx(&self) -> Option<f64> {
        self.semimajor_axis_approx().map(|a| a * (1.0 - self.eccentricity()))
    }

    fn distance_apogee(&self) -> f64 {
        self.semimajor_axis_ideal() * (1.0 + self.eccentricity())
    }

    fn distance_perigee(&self) -> f64 {
        self.semimajor_axis_ideal() * (1.0 - self.eccentricity())
    }

    fn semiminor_axis_approx(&self) -> Option<f64> {
        self.semimajor_axis_approx()
            .map(|semimajor| (semimajor.powi(2) - (self.eccentricity() * semimajor).powi(2)).sqrt())
    }

    fn semimajor_axis_approx(&self) -> Option<f64> {
        let mu = self.body().mu;
        let r = self.body().radius;
        let j2 = self.body().j2;
        let eccentricity = self.eccentricity(); // no unit
        let inclination = self.inclination().to_radians(); // deg in tle, we need radians

        let tmp = (1.0 - eccentricity.powi(2)).powf(-1.5) * (1.0 - 1.5 * inclination.sin().powi(2));
        let n = |a: f64| -> f64 {
            (mu / a.powi(3)).sqrt() * (1.0 + 1.5 * j2 * (r / a).powi(2) * tmp)
        };

        let n_deriv = |a: f64| -> f64 {
            mu.sqrt() * ((-1.5) * a.powf(-2.5) + (-3.5) * a.powf(-4.5) * 1.5 * j2 * r.powi(2) * tmp)
        };

        // let n_norad = (self.body().mu / self.semimajor_axis_ideal().powi(3)).sqrt();
        let n_norad = 2.0 * PI / self.period_of_revolution();
        let n_delta = |a: f64| -> f64 { n(a) - n_norad };

        let convergency = roots::SimpleConvergency {
            eps: 1.0e-14,
            max_iter: 50,
        };

        roots::find_root_newton_raphson(self.semimajor_axis_ideal(),
                                        &n_delta,
                                        &n_deriv,
                                        &convergency)
            .ok()
    }

    fn eccentric_anomaly(&self, time: DateTime<UTC>) -> Option<f64> {
        let delta_t = time.sub(self.timestamp());
        let delta_t_epoch = (delta_t.num_nanoseconds().unwrap() as f64) * 1.0e-9 / 86400f64;

        let eccentricity = self.eccentricity();
        // mean_anomaly is in degrees => convert to radians
        // mean_motion is rev*d^-1, so multiply with days since epoch and convert to rad
        let M = self.mean_anomaly().to_radians() + (self.mean_motion() * delta_t_epoch * 2.0 * PI);

        let e = |e: f64| -> f64 { e - eccentricity * e.sin() };
        let e_delta = |ei: f64| -> f64 { e(ei) - M };
        let e_deriv = |e: f64| -> f64 { 1f64 - eccentricity * e.cos() };

        let convergency = roots::SimpleConvergency {
            eps: 1.0e-12,
            max_iter: 50,
        };


        roots::find_root_newton_raphson(M, &e_delta, &e_deriv, &convergency).ok()
    }

    fn true_anomaly(&self, ea: f64) -> f64 {
        let eccentricity = self.eccentricity();
        let E = (ea * 0.5);

        let y = (1.0 + eccentricity).sqrt() * E.sin();
        let x = (1.0 - eccentricity).sqrt() * E.cos();

        2.0 * y.atan2(x)
    }

    fn longitude_ascending_node(&self, time: DateTime<UTC>) -> Option<f64> {
        let delta_t = time.sub(self.timestamp());
        let delta_t_epoch = (delta_t.num_nanoseconds().unwrap() as f64) * 1.0e-9 / 86400f64;

        let a = self.semimajor_axis_approx();
        let omega_dot = |a: f64| -> f64 {
            1.5 * self.body().j2
                * (self.body().radius / a).powi(2) 
                * self.mean_motion() * 2.0 * PI // n
                * (1.0 - self.eccentricity().powi(2)).powi(-2) *
            self.inclination().to_radians().cos()
        };

        a.map(|a| self.right_ascension().to_radians() - omega_dot(a) * delta_t_epoch)
    }

    fn argument_periapsis(&self, time: DateTime<UTC>) -> Option<f64> {
        let delta_t = time.sub(self.timestamp());
        let delta_t_epoch = (delta_t.num_nanoseconds().unwrap() as f64) * 1.0e-9 / 86400f64;

        let a = self.semimajor_axis_approx();
        let omega_dot = |a: f64| -> f64 {
            1.5 * self.body().j2
                * (self.body().radius / a).powi(2)
                * self.mean_motion() * 2.0 * PI // n
                * (1.0 - self.eccentricity().powi(2)).powi(-2) *
            (2.0 - 2.5 * self.inclination().to_radians().sin().powi(2))
        };


        a.map(|a| self.perigree().to_radians() + omega_dot(a) * delta_t_epoch)
    }
}

#[cfg(test)]
mod test {
    use tle;
    use satellite;
    use satellite::Satellite;
    use chrono::*;

    const DATA: &'static str = "ISS\n1 25544U 98067A   06040.85138889  .00012260  00000-0  \
                                86027-4 0  3194\n2 25544  51.6448 122.3522 0008835 257.3473 \
                                251.7436 15.74622749413094";

    const HA_DATA: &'static str = "MOLNIYA 3-41\n1 21706U 91065A   15110.48613875  .00000860  \
                                   00000-0  43336-1 0  9991\n2 21706  63.7252 112.9752 6936161 \
                                   280.1528 136.8490  2.04341486173495";

    #[test]
    fn test_satellite_calculations() {
        let tle = tle::TLE::new(&DATA.to_string()).unwrap();
        let satellite: tle::Satellite = satellite::Satellite::new(super::EARTH, tle);

        let time = "2015-05-04T15:46:25Z".parse::<DateTime<UTC>>().unwrap();
        // let time = satellite.timestamp();
        println!("eccentric anomaly: {:?}", satellite.eccentric_anomaly(time));
        println!("true anomaly: {:?}",
                 satellite.true_anomaly(satellite.eccentric_anomaly(time).unwrap()));

        println!("mean_motion: {}", satellite.mean_motion());
        println!("mean_anomaly_tle: {}", satellite.mean_anomaly());
        println!("eccentricity: {}", satellite.eccentricity());
        println!("SemiMaj: {}", satellite.semimajor_axis_ideal());
        println!("SemiMin: {}", satellite.semiminor_axis_ideal());
        println!("SemiMaj Approx: {:?}", satellite.semimajor_axis_approx());
        println!("SemiMin Approx: {:?}", satellite.semiminor_axis_approx());
        println!("distance_perigee: {}", satellite.distance_perigee());
        println!("distance_apogee: {}", satellite.distance_apogee());
        assert!(satellite.distance_perigee() == 6717901.720);
    }

}
