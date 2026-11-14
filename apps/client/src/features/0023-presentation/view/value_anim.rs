//! Animated value text rules after the OTClient `AnimatedText` behaviour: a number fades out over
//! the last part of its life and a new number joins a young one of the same colour on its tile.
use oteryn_session::PresentationColor;
use std::time::{Duration, Instant};

/// Seconds a value number rises and stays visible (the OTClient animated text duration).
pub(super) const VALUE_TEXT_SECONDS: f32 = 1.0;
/// The number is fully opaque until this fraction of its life has passed (1 / 1.2).
const FADE_START: f32 = VALUE_TEXT_SECONDS / 1.2;
/// A new number joins an existing one only while that one is younger than this (1 / 2.5).
const MERGE_WINDOW: Duration = Duration::from_millis(400);

/// Opacity percentage of a value number `age` seconds into its life.
pub(super) fn fade_percent(age: f32) -> u8 {
    if age <= FADE_START {
        return 100;
    }
    let left = 1.0 - (age - FADE_START) / (VALUE_TEXT_SECONDS - FADE_START);
    (left.clamp(0.0, 1.0) * 100.0).round() as u8
}

/// One plain value number: only a cue without a secondary number can merge.
#[derive(Clone, Copy)]
pub(super) struct Number {
    pub tile: (i32, i32, i16),
    pub color: PresentationColor,
    pub value: u64,
    pub plain: bool,
    pub received: Instant,
}

/// For each cue: `Some(total)` when its number is drawn (summed with every number that joined it),
/// `None` when it is not a value cue or joined an earlier one. `cues` are in arrival order.
pub(super) fn merge_plan(cues: &[Option<Number>]) -> Vec<Option<u64>> {
    let mut plan: Vec<Option<u64>> = cues.iter().map(|cue| cue.map(|n| n.value)).collect();
    for (index, cue) in cues.iter().enumerate() {
        let Some(new) = cue.filter(|n| n.plain) else {
            continue;
        };
        let host = (0..index).find(|&earlier| {
            plan[earlier].is_some()
                && cues[earlier].is_some_and(|old| {
                    old.plain
                        && old.tile == new.tile
                        && old.color == new.color
                        && new.received.saturating_duration_since(old.received) <= MERGE_WINDOW
                })
        });
        if let Some(host) = host {
            plan[host] = plan[host].map(|total| total.saturating_add(new.value));
            plan[index] = None;
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(tile: (i32, i32, i16), color: PresentationColor, value: u64, at: Instant) -> Option<Number> {
        Some(Number { tile, color, value, plain: true, received: at })
    }

    #[test]
    fn fade_holds_then_falls_to_zero() {
        assert_eq!(fade_percent(0.0), 100);
        assert_eq!(fade_percent(0.83), 100);
        assert_eq!(fade_percent(0.9165), 50);
        assert_eq!(fade_percent(1.0), 0);
        assert_eq!(fade_percent(2.0), 0);
    }

    #[test]
    fn young_same_colour_same_tile_numbers_merge() {
        let t = Instant::now();
        let red = PresentationColor::Red;
        let plan = merge_plan(&[
            number((1, 1, 7), red, 10, t),
            number((1, 1, 7), red, 5, t + Duration::from_millis(400)),
        ]);
        assert_eq!(plan, vec![Some(15), None]);
    }

    #[test]
    fn old_other_colour_other_tile_or_secondary_do_not_merge() {
        let t = Instant::now();
        let red = PresentationColor::Red;
        let later = t + Duration::from_millis(401);
        let with_secondary = Some(Number {
            tile: (1, 1, 7),
            color: red,
            value: 5,
            plain: false,
            received: t,
        });
        for other in [
            number((1, 1, 7), red, 5, later),
            number((1, 1, 7), PresentationColor::Blue, 5, t),
            number((2, 1, 7), red, 5, t),
            number((1, 1, 6), red, 5, t),
            with_secondary,
        ] {
            let plan = merge_plan(&[number((1, 1, 7), red, 10, t), other]);
            assert_eq!(plan, vec![Some(10), other.map(|n| n.value)]);
        }
    }

    #[test]
    fn a_third_number_joins_the_first_and_sums_saturate() {
        let t = Instant::now();
        let red = PresentationColor::Red;
        let plan = merge_plan(&[
            number((0, 0, 0), red, u64::MAX, t),
            None,
            number((0, 0, 0), red, 3, t + Duration::from_millis(100)),
            number((0, 0, 0), red, 4, t + Duration::from_millis(300)),
        ]);
        assert_eq!(plan, vec![Some(u64::MAX), None, None, None]);
    }
}
