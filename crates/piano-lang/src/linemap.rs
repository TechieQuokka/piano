//! Mapping between source lines and the timeline (line playback, highlighting).

use piano_core::{Score, Tick};

/// Tick range covered by events written on lines `from..=to` (`to = None`: to the end).
pub fn line_range(score: &Score, from: u32, to: Option<u32>) -> Option<(Tick, Tick)> {
    let to = to.unwrap_or(u32::MAX);
    let lines = from..=to;
    let notes = score
        .notes
        .iter()
        .filter(|n| lines.contains(&n.src.line))
        .map(|n| (n.start, n.end()));
    let pedals = score
        .pedals
        .iter()
        .filter(|p| lines.contains(&p.src.line))
        .map(|p| (p.at, p.at));
    notes
        .chain(pedals)
        .reduce(|(s1, e1), (s2, e2)| (s1.min(s2), e1.max(e2)))
}

/// Earliest tick of any note written at or after (`line`, `col`); `score.end` if none.
pub fn tick_at(score: &Score, line: u32, col: u32) -> Tick {
    score
        .notes
        .iter()
        .filter(|n| (n.src.line, n.src.col) >= (line, col))
        .map(|n| n.start)
        .min()
        .unwrap_or(score.end)
}

/// Source line of the latest note starting at or before `tick`.
pub fn line_at(score: &Score, tick: Tick) -> Option<u32> {
    score
        .notes
        .iter()
        .filter(|n| n.start <= tick)
        .max_by_key(|n| n.start)
        .map(|n| n.src.line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_piece;
    use piano_core::TICKS_PER_QUARTER as Q;

    #[test]
    fn ranges() {
        let c = compile_piece("C D\nE F\n// comment\nG");
        let s = &c.score;
        assert_eq!(line_range(s, 2, Some(2)), Some((2 * Q, 4 * Q)));
        assert_eq!(line_range(s, 2, None), Some((2 * Q, 5 * Q)));
        assert_eq!(line_range(s, 3, Some(3)), None);
        assert_eq!(tick_at(s, 2, 3), 3 * Q);
        assert_eq!(line_at(s, 4 * Q + 1), Some(4));
    }
}
