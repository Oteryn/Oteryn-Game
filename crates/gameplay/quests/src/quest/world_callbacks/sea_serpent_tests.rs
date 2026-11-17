#[test]
fn sea_serpent_telescope_is_registered_at_the_lookout() -> Result<(), String> {
    let callbacks = super::load()?;
    assert!(callbacks.iter().any(|callback| {
        callback.key == "oteryn:quest-callback/sea-serpent-telescope"
            && callback.trigger == super::Trigger::Use
            && callback.selector.native.as_ref().is_some_and(|selector| {
                selector.position.x == 31942
                    && selector.position.y == 31045
                    && selector.position.floor == -2
                    && selector.ordinal == 2
                    && selector.appearance == 938
            })
    }));
    Ok(())
}

#[test]
fn sea_serpent_telescope_lowers_against_imported_hunt_tracks() -> Result<(), String> {
    use crate::quest::{loader, sea_serpent};
    let state = loader::load_embedded_quest_state("sea-serpent-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = state.catalogue();
    let transition = catalogue
        .transition(sea_serpent::KEY)
        .ok_or("telescope transition")?;
    assert_eq!(transition.quest, sea_serpent::QUEST);
    for key in sea_serpent::TRACKS {
        assert_eq!(
            catalogue.track(key).map(|track| track.quest.as_str()),
            Some(sea_serpent::QUEST)
        );
    }
    Ok(())
}

#[test]
fn sea_serpent_haba_report_clears_pending_direction_and_resets_wrong_report() -> Result<(), String>
{
    use crate::quest::{
        loader,
        npc_rules::{NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger},
        sea_serpent,
    };
    use std::collections::BTreeMap;
    let state = loader::load_embedded_quest_state("sea-serpent-test")
        .map_err(|error| format!("{error:?}"))?;
    let npc = "oteryn:npc.captain_haba";
    let rules = NpcRuleCatalog::runtime()
        .and_then(|source| source.admitted_for_npc(npc, state.catalogue()))
        .map_err(|error| error.to_string())?
        .ok_or("admitted Captain Haba rules")?;
    let [line, _, direction, success, _, _] = sea_serpent::TRACKS;
    for (code, keyword) in [
        (1, "straight"),
        (2, "starboard"),
        (3, "larboard"),
        (4, "speed"),
    ] {
        for reported in 1..=4 {
            let before = i64::from(code != reported);
            let tracks = BTreeMap::from([
                (line.to_owned(), 1),
                (direction.to_owned(), reported),
                (success.to_owned(), before),
            ]);
            let planned = rules
                .plan_storage(&NpcRuleInput {
                    npc,
                    trigger: NpcRuleTrigger::Say,
                    message: keyword,
                    player_name: None,
                    topic: 0,
                    tracks: &tracks,
                    inventory: &Default::default(),
                    now_epoch_seconds: 0,
                    calendar_month: 1,
                    premium_current: true,
                    level: 61,
                    vocation: "knight",
                    sex: 0,
                    outfits: &BTreeMap::new(),
                })
                .map_err(|error| error.to_string())?
                .ok_or("report rule")?;
            assert!(
                planned
                    .track_changes
                    .iter()
                    .any(|change| { change.key == direction && change.after == 0 })
            );
            let after = planned
                .track_changes
                .iter()
                .find(|change| change.key == success)
                .map_or(before, |change| change.after);
            assert_eq!(after, i64::from(code == reported));
        }
    }
    Ok(())
}
