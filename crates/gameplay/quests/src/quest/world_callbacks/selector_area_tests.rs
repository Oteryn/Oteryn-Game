//! STAGPOS: the bounded STEP area selector and the two Order of the Stag
//! positional callbacks it activates.
//!
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/the_order_of_the_stag/isle_of_ada.lua:56-80 and
//! data-global/scripts/quests/the_order_of_the_stag/bounac_cathedral.lua:134-153.
use super::*;

const MINES: &str = "oteryn:quest-callback/order-of-the-stag-mines";
const CERTIFICATE: &str = "oteryn:quest-callback/order-of-the-stag-certificate";
const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";

fn embedded(key: &str) -> Result<Callback, String> {
    load()?
        .into_iter()
        .find(|callback| callback.key == key)
        .ok_or_else(|| format!("missing {key}"))
}

fn donor_tile(x: u16, y: u16, floor: i8) -> bool {
    (-7..=-6).contains(&floor)
        && (33866..=33877).contains(&x)
        && (32186..=32195).contains(&y)
        && !(floor == -7 && y == 32192 && (x == 33872 || x == 33873))
}

#[test]
fn stag_mines_area_selects_exactly_the_238_donor_tiles() -> Result<(), String> {
    let callback = embedded(MINES)?;
    assert_eq!(callback.trigger, Trigger::StepIn);
    assert_eq!(callback.storage.len(), 1);
    let stage = &callback.storage[0];
    assert_eq!(
        (
            stage.track.as_str(),
            stage.minimum,
            stage.maximum,
            stage.set
        ),
        (QUESTLINE, 4, 4, 5)
    );
    assert!(!stage.read_only && !stage.preserve);
    assert!(callback.item_reward().is_none() && callback.teleport.is_none());
    let mut selected = 0;
    for floor in -8..=-5 {
        for x in 33860..=33883 {
            for y in 32180..=32201 {
                let inside = donor_tile(x, y, floor);
                assert_eq!(
                    callback.selector.matches_position(Position { x, y, floor }),
                    inside,
                    "{x},{y},{floor}"
                );
                selected += usize::from(inside);
            }
        }
    }
    assert_eq!(selected, 238);
    Ok(())
}

#[test]
fn stag_certificate_is_a_position_only_stage_one_writ_grant() -> Result<(), String> {
    let callback = embedded(CERTIFICATE)?;
    assert_eq!(callback.trigger, Trigger::Use);
    let selector = &callback.selector;
    assert_eq!(
        selector.position,
        Some(Position {
            x: 32401,
            y: 32469,
            floor: -5
        })
    );
    assert!(selector.unique.is_none() && selector.action.is_none());
    assert!(selector.appearance.is_none() && selector.area.is_none());
    let stage = callback.storage.first().ok_or("stage guard")?;
    assert_eq!(callback.storage.len(), 1);
    assert_eq!(
        (
            stage.track.as_str(),
            stage.minimum,
            stage.maximum,
            stage.set
        ),
        (QUESTLINE, 1, 1, 2)
    );
    assert!(!stage.read_only && !stage.preserve);
    let reward = callback.item_reward().as_ref().ok_or("writ")?;
    assert_eq!(
        (reward.item.as_str(), reward.count),
        ("oteryn:item.tibia.i52586", 1)
    );
    assert!(reward.contents.is_empty() && reward.additional.is_empty());
    Ok(())
}

#[test]
fn bounded_area_selector_rejects_unbounded_or_ambiguous_areas() -> Result<(), String> {
    let callback = serde_json::to_value(embedded(MINES)?).map_err(|e| e.to_string())?;
    let input = serde_json::json!({"schema": 1, "callbacks": [callback]});
    parse(&input.to_string())?;
    for bad in 0..11 {
        let mut value = input.clone();
        let row = &mut value["callbacks"][0];
        match bad {
            0 => row["trigger"] = "use".into(),
            1 => row["selector"]["position"] = serde_json::json!({"x":33878,"y":32186,"floor":-7}),
            2 => {
                row["selector"]["area"]["excluded"][0] =
                    serde_json::json!({"x":33878,"y":32192,"floor":-7})
            }
            3 => {
                row["selector"]["area"]["excluded"][1] =
                    row["selector"]["area"]["excluded"][0].clone()
            }
            4 => row["selector"]["area"]["rectangles"][0]["x"] = serde_json::json!([33877, 33866]),
            5 => {
                row["selector"]["area"]["rectangles"][1]["floor"] =
                    row["selector"]["area"]["rectangles"][0]["floor"].clone()
            }
            6 => row["selector"]["area"]["rectangles"][0]["x"] = serde_json::json!([33000, 33877]),
            7 => {
                let rectangles = row["selector"]["area"]["rectangles"]
                    .as_array_mut()
                    .ok_or("rectangles")?;
                for floor in [-1, -2, -3] {
                    rectangles.push(serde_json::json!({"floor":floor,"x":[1,1],"y":[1,1]}));
                }
            }
            8 => {
                row["selector"]["position_aliases"] =
                    serde_json::json!([{"x":33867,"y":32186,"floor":-7}])
            }
            9 => row["selector"]["position"] = serde_json::json!({"x":33872,"y":32192,"floor":-7}),
            _ => row["selector"]["appearance"] = 100.into(),
        }
        assert!(
            parse(&value.to_string()).is_err(),
            "invalid area case {bad}"
        );
    }
    Ok(())
}
