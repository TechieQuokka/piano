//! Exact times and lengths in quarter notes. MusicXML durations are integer
//! divisions of a quarter that may change from part to part, and tuplets need
//! thirds and fifths, so floats would drift.

use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Sub};

/// A reduced fraction with a positive denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Q {
    n: i64,
    d: i64,
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Q {
    pub const ZERO: Q = Q { n: 0, d: 1 };

    pub fn new(n: i64, d: i64) -> Q {
        assert!(d != 0, "zero denominator");
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Q {
            n: s * n / g,
            d: s * d / g,
        }
    }

    pub fn int(n: i64) -> Q {
        Q { n, d: 1 }
    }

    pub fn numer(self) -> i64 {
        self.n
    }

    pub fn denom(self) -> i64 {
        self.d
    }

    pub fn is_integer(self) -> bool {
        self.d == 1
    }

    /// Largest integer not above `self`.
    pub fn floor(self) -> i64 {
        self.n.div_euclid(self.d)
    }

    pub fn to_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
}

impl Default for Q {
    fn default() -> Q {
        Q::ZERO
    }
}

impl Ord for Q {
    fn cmp(&self, other: &Q) -> Ordering {
        (i128::from(self.n) * i128::from(other.d)).cmp(&(i128::from(other.n) * i128::from(self.d)))
    }
}

impl PartialOrd for Q {
    fn partial_cmp(&self, other: &Q) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for Q {
    type Output = Q;
    fn add(self, o: Q) -> Q {
        Q::new(self.n * o.d + o.n * self.d, self.d * o.d)
    }
}

impl Sub for Q {
    type Output = Q;
    fn sub(self, o: Q) -> Q {
        Q::new(self.n * o.d - o.n * self.d, self.d * o.d)
    }
}

impl Mul for Q {
    type Output = Q;
    fn mul(self, o: Q) -> Q {
        Q::new(self.n * o.n, self.d * o.d)
    }
}

impl Div for Q {
    type Output = Q;
    fn div(self, o: Q) -> Q {
        Q::new(self.n * o.d, self.d * o.n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_is_exact() {
        let third = Q::new(1, 3);
        assert_eq!(third + third + third, Q::int(1));
        assert_eq!(Q::new(2, 4), Q::new(1, 2));
        assert_eq!(Q::new(1, -2), Q::new(-1, 2));
        assert!(Q::new(1, 3) < Q::new(1, 2));
        assert_eq!((Q::new(7, 2)).floor(), 3);
        assert_eq!(Q::new(3, 4) / Q::new(1, 4), Q::int(3));
    }
}
