//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/between_the_lines/actions_teleport_book_world.lua:20-48.
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position, Trigger};

#[test]
fn between_the_lines_preserves_the_executed_destination_and_scopes_shared_book_id()
-> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let third = callbacks
        .iter()
        .find(|callback| {
            callback.key == "oteryn:quest-callback/between-the-lines-third-labelled-to-two"
        })
        .ok_or("third-labelled book")?;
    // The donor comment says chapter III, but the executed literal is chapter II.
    // ANSWER 6589528748 preserves the executable donor when intent needs guessing.
    assert_eq!(
        third.teleport,
        Some(Position {
            x: 32526,
            y: 32513,
            floor: -9
        })
    );
    assert_eq!(third.selector.appearance, Some(51625));
    let returns: Vec<_> = callbacks
        .iter()
        .filter(|callback| {
            callback.quest == "oteryn:quest.between_the_lines_quest"
                && callback.selector.appearance == Some(51756)
        })
        .collect();
    assert_eq!(returns.len(), 3);
    for origin in [
        Position {
            x: 32526,
            y: 32512,
            floor: -9,
        },
        Position {
            x: 32552,
            y: 32594,
            floor: -10,
        },
        Position {
            x: 32585,
            y: 32557,
            floor: -11,
        },
    ] {
        let callback = returns
            .iter()
            .find(|callback| callback.selector.position == Some(origin))
            .ok_or("return origin")?;
        assert_eq!(callback.trigger, Trigger::Use);
        assert_eq!(
            callback.teleport,
            Some(Position {
                x: 32520,
                y: 32514,
                floor: -8
            })
        );
        assert!(callback.storage.is_empty());
    }
    Ok(())
}
