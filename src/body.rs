#[derive(Clone, PartialEq, Debug)]
pub struct Body {
    pub mu: f64,
    pub radius: f64,
    pub j2: f64,
    pub lambda: f64,
    pub we: f64,
}

pub const EARTH: Body = Body {
    mu: 398600_f64,
    radius: 6378.14_f64,
    j2: 0.00108263_f64,
    lambda: 99.281,
    we: 360.98564735,
};
