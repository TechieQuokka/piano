//! Musical time in integer ticks.

/// A position or length in musical time.
pub type Tick = u64;

/// Divisible by 2^6, 3, 5 and 7, so `/64` notes and common tuplets stay exact.
pub const TICKS_PER_QUARTER: Tick = 6720;

/// Length of a whole note.
pub const TICKS_PER_WHOLE: Tick = TICKS_PER_QUARTER * 4;

/// Allowed length denominators (`/1` .. `/64`).
pub const DENOMINATORS: [u8; 7] = [1, 2, 4, 8, 16, 32, 64];

/// Maximum number of dots a length may carry.
pub const MAX_DOTS: u8 = 2;

/// A written note length such as `/4` or `/8.`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Duration {
    denom: u8,
    dots: u8,
}

impl Duration {
    pub const QUARTER: Duration = Duration { denom: 4, dots: 0 };

    /// Returns `None` if `denom` is not a power of two in `1..=64` or `dots > MAX_DOTS`.
    pub fn new(denom: u8, dots: u8) -> Option<Duration> {
        (DENOMINATORS.contains(&denom) && dots <= MAX_DOTS).then_some(Duration { denom, dots })
    }

    pub fn denom(self) -> u8 {
        self.denom
    }

    pub fn dots(self) -> u8 {
        self.dots
    }

    /// Length in ticks, including dots.
    pub fn ticks(self) -> Tick {
        let base = TICKS_PER_WHOLE / Tick::from(self.denom);
        (0..=self.dots).map(|i| base >> i).sum()
    }

    /// Next shorter length (`/4` → `/8`), dots dropped; stays at `/64`.
    pub fn shorter(self) -> Duration {
        let denom = (self.denom * 2).min(64);
        Duration { denom, dots: 0 }
    }

    /// Next longer length (`/4` → `/2`), dots dropped; stays at `/1`.
    pub fn longer(self) -> Duration {
        let denom = (self.denom / 2).max(1);
        Duration { denom, dots: 0 }
    }
}

impl Default for Duration {
    fn default() -> Self {
        Duration::QUARTER
    }
}

impl std::fmt::Display for Duration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/{}", self.denom)?;
        for _ in 0..self.dots {
            f.write_str(".")?;
        }
        Ok(())
    }
}

/// A time signature such as `4/4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSig {
    pub beats: u8,
    pub unit: u8,
}

impl TimeSig {
    pub const COMMON: TimeSig = TimeSig { beats: 4, unit: 4 };

    /// Length of one bar in ticks.
    pub fn bar_ticks(self) -> Tick {
        TICKS_PER_WHOLE / Tick::from(self.unit) * Tick::from(self.beats)
    }
}

impl Default for TimeSig {
    fn default() -> Self {
        TimeSig::COMMON
    }
}

impl std::fmt::Display for TimeSig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.beats, self.unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_lengths() {
        assert_eq!(Duration::new(4, 0).unwrap().ticks(), TICKS_PER_QUARTER);
        assert_eq!(Duration::new(1, 0).unwrap().ticks(), TICKS_PER_WHOLE);
        assert_eq!(
            Duration::new(64, 0).unwrap().ticks(),
            TICKS_PER_QUARTER / 16
        );
    }

    #[test]
    fn dotted_lengths() {
        let q = TICKS_PER_QUARTER;
        assert_eq!(Duration::new(4, 1).unwrap().ticks(), q + q / 2);
        assert_eq!(Duration::new(4, 2).unwrap().ticks(), q + q / 2 + q / 4);
    }

    #[test]
    fn invalid_lengths() {
        assert!(Duration::new(3, 0).is_none());
        assert!(Duration::new(128, 0).is_none());
        assert!(Duration::new(4, 3).is_none());
    }

    #[test]
    fn tuplets_are_exact() {
        for n in [3, 5, 7] {
            assert_eq!(TICKS_PER_QUARTER % n, 0);
        }
    }

    #[test]
    fn shorter_longer_clamp() {
        assert_eq!(Duration::new(64, 0).unwrap().shorter().denom(), 64);
        assert_eq!(Duration::new(1, 0).unwrap().longer().denom(), 1);
        assert_eq!(Duration::QUARTER.shorter().denom(), 8);
    }

    #[test]
    fn bar_length() {
        assert_eq!(TimeSig::COMMON.bar_ticks(), TICKS_PER_WHOLE);
        assert_eq!(
            TimeSig { beats: 6, unit: 8 }.bar_ticks(),
            TICKS_PER_QUARTER * 3
        );
    }
}
