use super::*;
use oteryn_client_runtime::world_light::WorldClock;
#[test]
fn midnight_and_white_light() {
    let clock = WorldClock {
        tibian_minute: 0,
        light_level: 255,
        light_color: 215,
    };
    let mut state = None;
    protocol::apply(&mut state, clock);
    protocol::apply(
        &mut state,
        WorldClock {
            tibian_minute: 1439,
            ..clock
        },
    );
    assert_eq!(state.map(clock_text).as_deref(), Some("23:59"));
    assert_eq!(clock_text(clock), "00:00");
    assert_eq!(ambient_rgb(clock), [255; 3]);
    assert_eq!(
        ambient_rgb(WorldClock {
            light_level: 0,
            ..clock
        }),
        [0; 3]
    );
    assert_eq!(
        ambient_rgb(WorldClock {
            light_color: 180,
            ..clock
        }),
        [255, 0, 0]
    );
}
