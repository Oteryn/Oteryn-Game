//! Pinned Canary rafzane_elevator.lua: calls clear the departure tile;
//! riding uses the same platform batch with the source player as passenger.
use super::world_callbacks::{Callback, Position};

pub const UPPER_KEY: &str = "oteryn:quest-callback/goblin-merchant-elevator-upper";
pub const LOWER_KEY: &str = "oteryn:quest-callback/goblin-merchant-elevator-lower";
pub const UPPER: Position = Position {
    x: 33051,
    y: 32099,
    floor: -6,
};
pub const LOWER: Position = Position { floor: -7, ..UPPER };
const SOURCE: &str = "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/scripts/actions/other/rafzane_elevator.lua:1-105";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transform {
    pub position: Position,
    pub before: u16,
    pub after: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trip {
    pub from: Position,
    pub to: Position,
    pub evacuate: Position,
    pub items: [u16; 2],
    pub transforms: [Transform; 2],
    pub riding: bool,
    pub sound: &'static str,
    pub sound_position: Position,
}

pub fn trip(key: &str, platform_here: bool, rider_here: bool) -> Result<Trip, &'static str> {
    let down = match key {
        UPPER_KEY => platform_here,
        LOWER_KEY => !platform_here,
        _ => return Err("unknown elevator"),
    };
    if platform_here && !rider_here {
        return Err("Step inside the elevator to use it.");
    }
    Ok(if down {
        Trip {
            from: UPPER,
            to: LOWER,
            evacuate: Position { y: 32098, ..UPPER },
            items: [17938, 17939],
            transforms: [
                Transform {
                    position: UPPER,
                    before: 17939,
                    after: 17942,
                },
                Transform {
                    position: LOWER,
                    before: 17939,
                    after: 17943,
                },
            ],
            riding: platform_here,
            sound: "Srrrt!",
            sound_position: Position { x: 33052, ..UPPER },
        }
    } else {
        Trip {
            from: LOWER,
            to: UPPER,
            evacuate: Position { y: 32100, ..LOWER },
            items: [17938, 17943],
            // The pinned source intentionally applies both transforms on the upper tile.
            transforms: [
                Transform {
                    position: UPPER,
                    before: 17942,
                    after: 17939,
                },
                Transform {
                    position: UPPER,
                    before: 17943,
                    after: 17939,
                },
            ],
            riding: platform_here,
            sound: "Zrrrt!",
            sound_position: Position { x: 33052, ..LOWER },
        }
    })
}

pub fn callbacks() -> Result<Vec<Callback>, String> {
    let callbacks = [(UPPER_KEY, UPPER, 17940), (LOWER_KEY, LOWER, 17944)].map(|(key, pos, appearance)| {
        serde_json::json!({
            "key":key,"quest":"oteryn:quest.goblin_merchant_quest","trigger":"use",
            "selector":{"position":{"x":pos.x+1,"y":pos.y,"floor":pos.floor},"unique":null,"action":null,"appearance":appearance},
            "lever":null,"reversible":false,"passages":[],"teleport":null,
            "storage":[],"offerings":[],"return_on_failure":false,"failure_message":null,"sources":[SOURCE]
        })
    });
    super::world_callbacks::parse(
        &serde_json::json!({"schema":1,"callbacks":callbacks}).to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qm50_goblin_elevator_binds_both_source_winches_and_selects_call_or_ride()
    -> Result<(), String> {
        let callbacks = callbacks()?;
        assert_eq!(callbacks.len(), 2);
        for (callback, appearance, position) in
            [(&callbacks[0], 17940, UPPER), (&callbacks[1], 17944, LOWER)]
        {
            assert_eq!(
                callback.selector.position,
                Some(Position {
                    x: position.x + 1,
                    ..position
                })
            );
            assert_eq!(callback.selector.appearance, Some(appearance));
            assert!(!callback.has_personal_state());
        }
        let down = trip(UPPER_KEY, true, true)?;
        assert_eq!((down.from, down.to), (UPPER, LOWER));
        assert!(down.riding);
        let up = trip(LOWER_KEY, true, true)?;
        assert_eq!((up.from, up.to), (LOWER, UPPER));
        assert_eq!(trip(UPPER_KEY, false, false)?.from, LOWER);
        assert_eq!(trip(LOWER_KEY, false, false)?.from, UPPER);
        assert!(!trip(UPPER_KEY, false, false)?.riding);
        assert!(trip(UPPER_KEY, true, false).is_err());
        assert!(trip("unknown", false, false).is_err());
        assert_eq!(up.transforms[0].position, UPPER);
        assert_eq!(up.transforms[1].position, UPPER);
        Ok(())
    }
}
