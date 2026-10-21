//! Same-floor Chebyshev reach in the native coordinate frame.

/// Whether the two positions share a floor and differ by at most one tile per axis.
pub fn within_reach(actor: (i32, i32, i32), target: (i32, i32, i32)) -> bool {
    actor.2 == target.2
        && (i64::from(actor.0) - i64::from(target.0)).abs() <= 1
        && (i64::from(actor.1) - i64::from(target.1)).abs() <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reach_is_one_tile_on_the_actors_floor() {
        assert!(within_reach((10, 10, -7), (11, 9, -7)));
        assert!(within_reach((10, 10, -7), (10, 10, -7)));
        assert!(!within_reach((10, 10, -7), (12, 10, -7)));
        assert!(!within_reach((10, 10, -7), (10, 10, -6)));
        assert!(!within_reach((i32::MIN, 0, 7), (i32::MAX, 0, 7)));
    }
}
