use super::*;

type Exchange = (&'static str, u32, u16, u32);

fn check_exchanges(npc: &str, expected: &[Exchange]) -> Result<(), String> {
    let callbacks = load()?;
    let item = |id| format!("oteryn:item.tibia.i{id}");
    for &(keyword, input, cost, output) in expected {
        let callback = callbacks
            .iter()
            .find(|row| row.key == format!("oteryn:quest-reward/{npc}-exchange-{output}"))
            .ok_or_else(|| format!("missing {npc} exchange for {keyword}"))?;
        assert_eq!(callback.npc, format!("oteryn:npc.{npc}"));
        assert!(callback.is_paid_exchange());
        assert_eq!(
            callback.keywords,
            [if npc == "jorge" { keyword } else { "yes" }]
        );
        let paid = if npc == "cledwyn" {
            assert_eq!(callback.take.len(), 2);
            assert_eq!(callback.take[0].item, item(input));
            assert_eq!(callback.take[0].count, 1);
            (22516, &callback.take[1])
        } else {
            assert_eq!(callback.take.len(), 1);
            (input, &callback.take[0])
        };
        assert_eq!(paid.1.item, item(paid.0));
        assert_eq!(paid.1.count, cost);
        let reward = callback.give.as_ref().ok_or("missing reward")?;
        assert_eq!(reward.item, item(output));
        assert_eq!(reward.count, 1);
    }
    Ok(())
}

#[test]
fn cledwyn_exchange_recipes() -> Result<(), String> {
    check_exchanges(
        "cledwyn",
        &[
            ("pendulet", 29429, 2, 30344),
            ("sleep shawl", 29428, 2, 30342),
            ("blister ring", 31621, 2, 31557),
            ("theurgic amulet", 30401, 2, 30403),
            ("ring of souls", 32636, 2, 32621),
            ("spiritthorn ring", 39179, 5, 39177),
            ("alicorn ring", 39182, 5, 39180),
            ("arcanomancer sigil", 39185, 5, 39183),
            ("arboreal ring", 39188, 5, 39187),
            ("turtle amulet", 39235, 2, 39233),
        ],
    )
}

#[test]
fn gnomally_exchange_recipes() -> Result<(), String> {
    check_exchanges(
        "gnomally",
        &[
            ("muck", 16128, 8, 16101),
            ("mission", 16128, 10, 16242),
            ("lamp", 16128, 15, 16094),
            ("backpack", 16128, 15, 16099),
            ("gill gugel", 16129, 10, 16104),
            ("gill coat", 16129, 10, 16105),
            ("gill legs", 16129, 10, 16106),
            ("spellbook", 16129, 10, 16107),
            ("prismatic helmet", 16129, 10, 16109),
            ("prismatic armor", 16129, 10, 16110),
            ("prismatic legs", 16129, 10, 16111),
            ("prismatic boots", 16129, 10, 16112),
            ("prismatic shield", 16129, 10, 16116),
            ("iron loadstone", 16129, 20, 16153),
            ("glow wine", 16129, 20, 16154),
        ],
    )
}

#[test]
fn jorge_exchange_recipes() -> Result<(), String> {
    check_exchanges(
        "jorge",
        &[
            ("abacus", 19083, 20, 19151),
            ("assassin doll", 19083, 20, 28897),
            ("bag of oriental spices", 19083, 20, 23682),
            ("bookworm doll", 19083, 20, 28895),
            ("citizen doll", 19083, 20, 43511),
            ("crimson doll", 19083, 20, 25981),
            ("doll of durin the almighty", 19083, 20, 23679),
            ("dragon eye", 19083, 20, 22027),
            ("dragon goblet", 19083, 20, 31265),
            ("draken doll", 19083, 20, 25979),
            ("emblem", 19083, 20, 25980),
            ("encyclopedia", 19083, 20, 23678),
            ("friendship amulet", 19083, 20, 19153),
            ("frozen heart", 19083, 20, 19156),
            ("golden falcon", 19083, 20, 28896),
            ("golden newspaper", 19083, 20, 23681),
            ("goromaphone", 19083, 20, 34210),
            ("hand puppets", 19083, 20, 23676),
            ("imortus", 19083, 20, 23683),
            ("jade amulet", 19083, 20, 31268),
            ("key of numerous locks", 19083, 20, 19152),
            ("little adventurer doll", 19083, 20, 37058),
            ("loremaster doll", 19083, 20, 31267),
            ("lucky clover amulet", 19083, 20, 37059),
            ("mathmaster shield", 19083, 20, 25982),
            ("medusa skull", 19083, 20, 23680),
            ("music box", 19083, 20, 23677),
            ("noble sword", 19083, 20, 22028),
            ("norseman doll", 19083, 20, 19150),
            ("old radio", 19083, 20, 28894),
            ("orc's jaw shredder", 19083, 20, 19155),
            ("phoenix statue", 19083, 20, 22026),
            ("pigeon trophy", 19083, 20, 31266),
            ("shield of destiny", 19083, 20, 43517),
            ("shield of endless search", 19083, 20, 37060),
            ("the mexcalibur", 19083, 20, 19154),
            ("tibiora's box", 19083, 20, 43510),
        ],
    )
}

#[test]
fn ruprecht_exchange_recipes() -> Result<(), String> {
    check_exchanges(
        "ruprecht",
        &[
            ("gingerbreadman", 6526, 1, 6500),
            ("christmas cookie tray", 6526, 1, 20310),
            ("gingerbread recipe", 6526, 10, 174),
            ("jewel case", 6526, 25, 7527),
            ("santa hat", 6526, 50, 6531),
            ("santa backpack", 6526, 75, 10346),
            ("snow flake tapestry", 6526, 75, 20315),
            ("santa doll", 6526, 100, 6511),
            ("snowman doll", 6526, 150, 10339),
            ("snow globe", 6526, 150, 20311),
            ("frazzlemaw santa", 6526, 250, 20308),
            ("leaf golem santa", 6526, 250, 20309),
            ("santa music box", 6526, 250, 20313),
            ("santa teddy", 6526, 500, 10338),
            ("maxxen santa", 6526, 250, 21952),
            ("present bag", 6496, 1, 6526),
            ("ferumbras' teddy santa", 6526, 250, 22879),
            ("nightmare beast santa", 6526, 250, 29947),
            ("orclops santa", 6526, 250, 24394),
            ("raccoon santa", 6526, 250, 35692),
            ("santa fox", 6526, 250, 27591),
            ("santa leech", 6526, 250, 32746),
        ],
    )
}

#[test]
fn storkus_rank_hand_ins() -> Result<(), String> {
    let callbacks = load()?;
    let rank = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/rank";
    let door = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/door";
    let prerequisite = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    assert_eq!(catalogue.track(rank).ok_or("rank track")?.initial, -1);
    assert_eq!(catalogue.track(door).ok_or("door track")?.initial, -1);
    assert!(catalogue.track(prerequisite).is_some());

    for (previous, (cost, xp)) in (0_i64..).zip([
        (1, 1000),
        (4, 5000),
        (5, 10000),
        (10, 20000),
        (30, 50000),
        (50, 100000),
    ]) {
        let next = previous + 1;
        let callback = callbacks
            .iter()
            .find(|row| row.key == format!("oteryn:quest-reward/storkus-rank-{next}"))
            .ok_or_else(|| format!("missing Storkus rank {next}"))?;
        assert_eq!(callback.npc, "oteryn:npc.storkus");
        assert_eq!(callback.keywords, ["yes"]);
        assert_eq!(callback.take.len(), 1);
        assert_eq!(callback.take[0].item, "oteryn:item.tibia.i8192");
        assert_eq!(callback.take[0].count, cost);
        assert_eq!(callback.experience, Some(xp));
        assert_eq!(callback.requires.len(), 1);
        assert_eq!(callback.requires[0].track, prerequisite);
        assert_eq!(callback.requires[0].minimum, 11);
        assert_eq!(callback.storage[0].track, rank);
        assert_eq!(
            callback.storage[0].minimum,
            if previous == 0 { -1 } else { previous }
        );
        assert_eq!(callback.storage[0].maximum, previous);
        assert_eq!(callback.storage[0].set, next);
        if next == 6 {
            let reward = callback.give.as_ref().ok_or("missing final reward")?;
            assert_eq!(reward.item, "oteryn:item.tibia.i8191");
            assert_eq!(reward.count, 1);
            assert_eq!(callback.storage[1].track, door);
            assert_eq!(
                (callback.storage[1].minimum, callback.storage[1].maximum),
                (-1, 1)
            );
            assert_eq!(callback.storage[1].set, 1);
        } else {
            assert!(callback.give.is_none());
            assert_eq!(callback.storage.len(), 1);
        }
    }
    Ok(())
}

#[test]
fn jorge_matches_a_souvenir_inside_the_live_request() -> Result<(), String> {
    let callbacks = load()?;
    let music_box = callbacks
        .iter()
        .find(|callback| {
            callback.npc == "oteryn:npc.jorge"
                && callback.keywords.iter().any(|word| word == "music box")
        })
        .ok_or("music box souvenir")?;
    assert!(music_box.matches_keyword("I'd like a music box"));
    assert!(!music_box.matches_keyword("I'd like an xmusic box"));
    Ok(())
}
