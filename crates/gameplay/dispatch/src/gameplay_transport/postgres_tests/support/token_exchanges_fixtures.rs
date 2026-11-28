use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    reward_callbacks,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

const INQUISITION: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const RANK: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/rank";
const DOOR: &str = "oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/door";

async fn mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    count: u16,
    nonce: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        nonce,
        true,
    )
    .await
}

async fn track(h: &Harness, key: &str) -> TestResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(key).fetch_optional(&h.pool).await?)
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}

async fn stow_completed_exchanges(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    mint(owner, admitted, 2854, 1, 900_002).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("main backpack")?;
    let spare = backpack
        .entries
        .iter()
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i2854")
        .ok_or("spare backpack")?
        .item
        .item_instance_id;
    for (index, entry) in backpack.entries.iter().enumerate() {
        let item = entry.item.item_instance_id;
        if item != spare {
            super::qengine_rewards_fixtures::into_bag(
                owner,
                admitted,
                item,
                spare,
                925_000 + u64::try_from(index)?,
            )
            .await?;
        }
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::qengine_rewards_fixtures::wire_conversation;
    use super::questpc7_outfit_items_fixtures::quantity;

    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("TOKEN_EXCHANGES POSTGRES_STARTED version={version}");
    super::qd_world_fixtures::select_talk_capabilities(owner, admitted, &[3, 6, 25])?;
    let [cledwyn, gnomally, jorge, ruprecht, storkus] = super::classic_quests_fixtures::install(
        owner,
        admitted,
        [
            "oteryn:npc.cledwyn",
            "oteryn:npc.gnomally",
            "oteryn:npc.jorge",
            "oteryn:npc.ruprecht",
            "oteryn:npc.storkus",
        ],
    )
    .await?;
    let callbacks = reward_callbacks::load()?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native item policy")?;
    for callback in callbacks.iter().filter(|row| {
        row.key.split('/').next_back().is_some_and(|name| {
            [
                "cledwyn-exchange-",
                "gnomally-exchange-",
                "jorge-exchange-",
                "ruprecht-exchange-",
                "vampire-hunter-rank-",
            ]
            .iter()
            .any(|prefix| name.starts_with(prefix))
        })
    }) {
        for key in callback
            .take
            .iter()
            .map(|item| &item.item)
            .chain(callback.give.iter().map(|item| &item.item))
        {
            let policy = native
                .item_policy(key, "definition-r1")
                .ok_or_else(|| format!("missing native item policy for {key}"))?;
            assert!(policy.record().admission.materializable, "{key}");
            assert!(
                policy.record().admission.legal_destinations.contains(
                    &crate::content::native_gameplay::NativeItemDestination::CharacterInventory
                ),
                "{key}"
            );
        }
    }
    let find = |name: &str| {
        callbacks
            .iter()
            .find(|row| row.key == format!("oteryn:quest-reward/{name}"))
            .ok_or_else(|| format!("missing token callback {name}"))
    };

    let mut regressions = Vec::new();
    let mut shop_windows = Vec::new();
    for (index, text) in ["hi", "trade silver", "silver", "bye"]
        .into_iter()
        .enumerate()
    {
        use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
        use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;

        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 949_980 + u64::try_from(index)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: cledwyn,
                    text: text.into(),
                },
            )
            .await;
        if text.contains("silver") {
            shop_windows.push(outcome.window);
        }
    }
    for (message, window) in ["trade silver", "silver"].into_iter().zip(shop_windows) {
        let open = window.as_ref().and_then(|window| window.open);
        if open.is_some() {
            regressions.push(format!(
                "Cledwyn {message:?} opened an unusable silver-token shop"
            ));
        } else if window.is_some_and(|window| !window.offers.is_empty()) {
            regressions.push(format!("Cledwyn {message:?} showed silver offers as gold"));
        }
    }

    for (offset, (text, opens_shop)) in [
        ("hi", false),
        ("items", false),
        ("no thanks", false),
        ("yes", false),
        ("mission items", false),
        ("yes please", true),
        ("yes", false),
        ("bye", false),
    ]
    .into_iter()
    .enumerate()
    {
        use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
        use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;

        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 949_960 + u64::try_from(offset)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: gnomally,
                    text: text.into(),
                },
            )
            .await;
        let open = outcome
            .window
            .and_then(|window| window.open.map(|(npc, _)| (npc, window.offers.len())));
        if opens_shop && !open.is_some_and(|(npc, offers)| npc == gnomally && offers > 0)
            || !opens_shop && open.is_some()
        {
            regressions.push(format!(
                "Gnomally {text:?}: unexpected shop window {open:?}"
            ));
        }
    }

    let supply = find("l9-gnomish-supply")?;
    let initial_supply = quantity(h, 15698).await?;
    assert_eq!(quantity(h, 16128).await?, 0);
    let talk = super::qengine_rewards_fixtures::talk;
    for (command, message) in [(949_990, "hi"), (949_991, "supply"), (949_992, "yes")] {
        talk(owner, admitted, gnomally, command, message).await?;
    }
    assert_eq!(quantity(h, 15698).await?, initial_supply);
    mint(owner, admitted, 16128, 2, 949_993).await?;
    talk(owner, admitted, gnomally, 949_994, "yes").await?;
    if quantity(h, 15698).await? != initial_supply || quantity(h, 16128).await? != 2 {
        regressions.push("Gnomally accepted a stale quote after insufficient funds".to_owned());
    } else {
        talk(owner, admitted, gnomally, 949_995, "supply").await?;
        talk(owner, admitted, gnomally, 949_996, "yes").await?;
    }
    assert_eq!(quantity(h, 15698).await?, initial_supply + 1);
    assert_eq!(quantity(h, 16128).await?, 0);
    assert!(supply.is_paid_exchange());
    talk(owner, admitted, gnomally, 949_997, "bye").await?;

    for (npc, key, words, payment, cost, output, ingredient) in [
        (
            cledwyn,
            "cledwyn-exchange-30344",
            vec!["hi", "enchant", "pendulet", "yes"],
            22516,
            2,
            30344,
            Some(29429),
        ),
        (
            cledwyn,
            "cledwyn-exchange-39187",
            vec!["hi", "enchant", "arboreal ring", "yes"],
            22516,
            5,
            39187,
            Some(39188),
        ),
        (
            gnomally,
            "gnomally-exchange-16101",
            vec!["hi", "muck", "yes"],
            16128,
            8,
            16101,
            None,
        ),
        (
            gnomally,
            "gnomally-exchange-16104",
            vec!["hi", "gill gugel", "yes"],
            16129,
            10,
            16104,
            None,
        ),
        (
            jorge,
            "jorge-exchange-19151",
            vec!["hi", "souvenir", "abacus"],
            19083,
            20,
            19151,
            None,
        ),
        (
            ruprecht,
            "ruprecht-exchange-6526",
            vec!["hi", "present bag", "yes"],
            6496,
            1,
            6526,
            None,
        ),
    ] {
        let callback = find(key)?;
        let initial_payment = quantity(h, payment).await?;
        let initial_output = quantity(h, output).await?;
        let mut nonce = 100_000 + u64::from(output) * 10;
        if let Some(ingredient) = ingredient {
            mint(owner, admitted, ingredient, 1, nonce).await?;
            nonce += 1;
        }
        assert_eq!(
            wire_conversation(owner, admitted, npc, callback, false, words.clone())
                .await?
                .0,
            NpcIntentDisposition::Rejected,
            "{key} without payment"
        );
        assert_eq!(quantity(h, payment).await?, initial_payment);
        assert_eq!(quantity(h, output).await?, initial_output);
        if let Some(ingredient) = ingredient {
            assert_eq!(
                quantity(h, ingredient).await?,
                1,
                "a refusal retains the discharged item"
            );
        }
        for purchase in 1..=2 {
            if purchase == 2
                && let Some(ingredient) = ingredient
            {
                mint(owner, admitted, ingredient, 1, nonce + 2).await?;
            }
            mint(owner, admitted, payment, cost, nonce).await?;
            nonce += 1;
            assert_eq!(
                wire_conversation(owner, admitted, npc, callback, false, words.clone())
                    .await?
                    .0,
                NpcIntentDisposition::Ok,
                "{key} purchase {purchase}"
            );
            assert_eq!(quantity(h, payment).await?, initial_payment);
            assert_eq!(quantity(h, output).await?, initial_output + purchase);
            if let Some(ingredient) = ingredient {
                assert_eq!(quantity(h, ingredient).await?, 0);
            }
        }
    }
    let cookie = find("ruprecht-exchange-20310")?;
    assert_eq!(
        wire_conversation(
            owner,
            admitted,
            ruprecht,
            cookie,
            false,
            vec!["hi", "christmas cookie tray", "yes"]
        )
        .await?
        .0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(quantity(h, 6526).await?, 1);
    assert_eq!(quantity(h, 20310).await?, 1);

    let first = find("vampire-hunter-rank-1")?;
    let words = vec!["hi", "vampire lord token", "yes"];
    mint(owner, admitted, 8192, 100, 900_000).await?;
    wire_conversation(
        owner,
        admitted,
        storkus,
        first,
        false,
        vec!["hi", "vampire lord token"],
    )
    .await?;
    assert_eq!(quantity(h, 8192).await?, 100, "Inquisition prerequisite");
    assert_eq!(track(h, RANK).await?, None);
    // Represent a character already eligible from the parent quest.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.the_inquisition_quest',11,2)")
        .bind(id(41).as_slice()).bind(INQUISITION).execute(&mut *tx).await?;
    tx.commit().await?;
    owner.admit_quest_session(admitted).await;
    owner.admit_progression_session(admitted).await;

    wire_conversation(
        owner,
        admitted,
        storkus,
        first,
        false,
        vec!["hi", "vampire lord token", "yes mission"],
    )
    .await?;
    assert_eq!(quantity(h, 8192).await?, 100, "yes mission spent tokens");
    assert_eq!(track(h, RANK).await?, None, "yes mission advanced rank");

    let mut tokens = 100;
    let mut xp = experience(h).await?;
    for (index, (cost, earned)) in [
        (1, 1000),
        (4, 5000),
        (5, 10000),
        (10, 20000),
        (30, 50000),
        (50, 100000),
    ]
    .into_iter()
    .enumerate()
    {
        let rank = i64::try_from(index)? + 1;
        let callback = find(&format!("vampire-hunter-rank-{rank}"))?;
        assert_eq!(
            wire_conversation(owner, admitted, storkus, callback, false, words.clone())
                .await?
                .0,
            NpcIntentDisposition::Ok,
            "Storkus rank {rank}"
        );
        tokens -= cost;
        xp += earned;
        assert_eq!(quantity(h, 8192).await?, tokens);
        assert_eq!(track(h, RANK).await?, Some(rank));
        assert_eq!(experience(h).await?, xp);
        assert_eq!(track(h, DOOR).await?, (rank == 6).then_some(1));
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let state = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.meets_door_storage(41731), rank == 6);
    }
    assert_eq!(quantity(h, 8191).await?, 1);
    mint(owner, admitted, 8192, 50, 900_001).await?;
    wire_conversation(
        owner,
        admitted,
        storkus,
        find("vampire-hunter-rank-6")?,
        false,
        vec!["hi", "vampire lord token"],
    )
    .await?;
    assert_eq!(quantity(h, 8192).await?, 50);
    assert_eq!(quantity(h, 8191).await?, 1);
    assert_eq!(experience(h).await?, xp);
    let achievements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/razing'"
    ).fetch_one(&h.pool).await?;
    assert_eq!(
        achievements, 1,
        "the final rank and its replay grant Razing! once"
    );
    // Keep the verified rewards while making room for the next dialogue cases.
    stow_completed_exchanges(owner, admitted).await?;
    for (index, (npc, key, words, inputs, output, expected)) in [
        (
            cledwyn,
            "cledwyn-exchange-30344",
            vec!["hi", "enchant", "pendulet", "no", "yes"],
            vec![(29429, 1), (22516, 2)],
            30344,
            0,
        ),
        (
            cledwyn,
            "cledwyn-exchange-30344",
            vec!["hi", "enchant", "pendulet", "yes, silver tokens"],
            vec![(29429, 1), (22516, 2)],
            30344,
            0,
        ),
        (
            cledwyn,
            "cledwyn-exchange-30344",
            vec!["hi", "enchant", "pendulet", "enchant tokens", "yes"],
            vec![(29429, 1), (22516, 2)],
            30344,
            1,
        ),
        (
            cledwyn,
            "cledwyn-exchange-30344",
            vec!["hi", "enchant", "pendulet", "yes silver", "no", "yes"],
            vec![(29429, 1), (22516, 2)],
            30344,
            0,
        ),
        (
            cledwyn,
            "cledwyn-exchange-30344",
            vec!["hi", "enchant", "pendulet", "yes silver", "yes"],
            vec![(29429, 1), (22516, 2)],
            30344,
            1,
        ),
        (
            gnomally,
            "gnomally-exchange-16101",
            vec!["hi", "muck", "yes, mission items"],
            vec![(16128, 8)],
            16101,
            0,
        ),
        (
            gnomally,
            "gnomally-exchange-16101",
            vec!["hi", "muck", "yes, relations"],
            vec![(16128, 8)],
            16101,
            0,
        ),
        (
            gnomally,
            "l9-gnomish-supply",
            vec!["hi", "supply", "no, yes"],
            vec![(16128, 2)],
            15698,
            1,
        ),
        (
            gnomally,
            "l9-gnomish-supply",
            vec!["hi", "muck", "supply", "yes"],
            vec![(16128, 8)],
            15698,
            1,
        ),
        (
            gnomally,
            "l9-gnomish-supply",
            vec!["hi", "supply", "no", "yes"],
            vec![(16128, 2)],
            15698,
            0,
        ),
        (
            jorge,
            "jorge-exchange-28897",
            vec!["hi", "souvenir", "music box or assassin doll"],
            vec![(19083, 20)],
            28897,
            1,
        ),
        (
            gnomally,
            "gnomally-exchange-16094",
            vec!["hi", "muck", "lamp", "yes"],
            vec![(16128, 15)],
            16094,
            1,
        ),
        (
            jorge,
            "jorge-exchange-23677",
            vec!["hi", "I'd like a souvenir", "music box"],
            vec![(19083, 20)],
            23677,
            1,
        ),
        (
            jorge,
            "jorge-exchange-19151",
            vec!["hi", "souvenir", "music box or abacus"],
            vec![(19083, 20)],
            19151,
            1,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let initial = quantity(h, output).await?;
        for (item, count) in inputs {
            mint(
                owner,
                admitted,
                item,
                count,
                950_000 + u64::from(item) * 10 + u64::try_from(index)?,
            )
            .await?;
        }
        let description = words.join(" / ");
        wire_conversation(owner, admitted, npc, find(key)?, false, words).await?;
        let actual = quantity(h, output).await? - initial;
        if actual != expected {
            regressions.push(format!("{description}: expected {expected}, got {actual}"));
        }
    }
    assert!(regressions.is_empty(), "{regressions:#?}");
    // An existing Bigfoot recruit can trade a quoted quantity across a rank boundary.
    const LINE: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/quest_line";
    const RENOWN: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/rank";
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (key, value) in [(LINE, 25_i64), (RENOWN, 22)] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.bigfoot_s_burden_quest',$3,2) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
            .bind(id(41).as_slice()).bind(key).bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner.admit_quest_session(admitted).await;
    mint(owner, admitted, 16128, 2, 998_000).await?;
    let before = quantity(h, 16128).await?;
    for (i, word) in ["bye", "hi", "relations", "yes", "2", "yes"]
        .iter()
        .enumerate()
    {
        super::qengine_rewards_fixtures::talk(owner, admitted, gnomally, 998_001 + i as u64, word)
            .await?;
    }
    assert_eq!(track(h, RENOWN).await?, Some(32));
    assert_eq!(track(h, LINE).await?, Some(26));
    assert_eq!(quantity(h, 16128).await?, before - 2);
    super::qengine_rewards_fixtures::talk(owner, admitted, gnomally, 998_006, "yes").await?;
    assert_eq!(quantity(h, 16128).await?, before - 2);
    assert_eq!(track(h, RENOWN).await?, Some(32));

    Ok(())
}

pub(super) async fn seed_unfinished_exchanges(h: &Harness, variant: &str) -> TestResult {
    for (quest, train, published) in [
        ("oteryn:quest.bigfoot_s_burden_quest", "91fe6ba3798095b75eed90c90ed2db0985dae1549dd4923ccc53d070f31fe542", Some("f4aea058bd9f28e4c1f12b44413cc4f37ad6ad8488246bc66626f8d7b497ddc8")),
        ("oteryn:quest.hunting_for_tokens", "697aa47c5147077c4931644597aefe3ef96698212b6e1f4c84143d90d32a9592", Some("f68593b143c5de45970b3a2375e9264d9781bc7968f59cc67c218b4e66cd05b3")),
        ("oteryn:quest.present_quest", "5d491ab6c563697f5d5318b0b9c3464878c505c7cb10c7a9f116cc712bddc859", None),
        ("oteryn:quest.rift_warrior_outfits_quest", "2bea7fe1165bfca2838ecf0913dee9dbbe253b05e001f367ae19944a020abc49", Some("c78651384e378afe5e740ecccbbd20354a80c4baf1abf6190fae1d21247c1a94")),
        ("oteryn:quest.vampire_hunter_quest", "1c364b5af97b6e6a9bea86be0e9bed30a8778c234e49af4bf81d23977950931e", None),
    ] {
        if let Some(pin) = if variant == "train-save" { Some(train) } else { published } {
            super::qc_pinned_world::seed_unfinished_state(h, quest, pin).await?;
        }
    }
    Ok(())
}
