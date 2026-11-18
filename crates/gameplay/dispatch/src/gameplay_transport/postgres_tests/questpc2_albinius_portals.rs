//! Four Albinius portal offerings earned after the real five-tome contribution.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, npc_rules::NpcRuleCatalog};

const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const CONTACTMAN: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/formorgar_mines_door";
const FISH: &str = "oteryn:item.tibia.i3578";
const ICE: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/access_ice";
static ICE_REVISION: std::sync::OnceLock<(NpcRuleCatalog, [u8; 32])> = std::sync::OnceLock::new();

const PREFIX: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/";
const PORTALS: [(&str, &str, &str, &str); 4] = [
    (
        "holy portal",
        "access_golden",
        "oteryn:item.tibia.i18929",
        "incantation notes",
    ),
    (
        "energy portal",
        "access_violet",
        "oteryn:item.tibia.i17462",
        "marsh stalker feathers",
    ),
    (
        "earth portal",
        "access_earth",
        "oteryn:item.tibia.i10296",
        "acorns",
    ),
    (
        "death portal",
        "access_death",
        "oteryn:item.tibia.i11481",
        "pelvis bones",
    ),
];

#[test]
fn questpc2_albinius_portals_require_fifty_exact_items_and_live_confirmation() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-portals-exact")
}

#[test]
fn questpc2_albinius_portals_combined_source_ifs_commit_all_four_offerings() -> TestResult {
    super::qengine_rewards::scenario("questpc2-albinius-portals-combined")
}

#[test]
fn fknow_ice_portal_accepts_existing_albinius_progress() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run("fknow-ice-portal", || {
        let baseline = load_embedded_quest_state("content-1").map_err(debug)?;
        let hash = baseline
            .catalogue()
            .definition_hash(QUEST)
            .ok_or("baseline pin")?;
        let source = NpcRuleCatalog::runtime().map_err(debug)?;
        let mut document = source
            .program(QUEST)
            .ok_or("Albinius programme")?
            .document
            .clone();
        let ice: oteryn_gameplay_quests::quest::npc_rules::NpcRuleDocument =
            serde_json::from_str(include_str!("fknow/ice_portal.json"))?;
        for rule in &mut document.rules {
            rule.exclude_keywords
                .retain(|keyword| keyword != "ice portal");
        }
        document.tracks.extend(ice.tracks);
        document.tracks.sort_by(|a, b| a.key.cmp(&b.key));
        let before_holy = document
            .rules
            .iter()
            .position(|rule| rule.key.ends_with("/holy-offer"))
            .ok_or("holy offering")?;
        document.rules.splice(before_holy..before_holy, ice.rules);
        let successor = source
            .with_fixture_document(&serde_json::to_string(&document)?)
            .map_err(debug)?;
        let (successor, _) = ICE_REVISION.get_or_init(|| (successor, hash));
        successor.in_fixture_runtime(|| {
            super::quest_world_callbacks::scenario(
                "questpc2-albinius-portals-ice",
                Position {
                    x: 33294,
                    y: 31677,
                    floor: -15,
                },
                false,
            )
        })
    })
}

async fn carried(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.lifecycle=1")
        .bind(id(41).as_slice()).bind(item).fetch_one(&h.pool).await?)
}

async fn give(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: &str,
    count: u16,
    n: u64,
) -> TestResult {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    let position = Position {
        x: u16::try_from(at.x)?,
        y: u16::try_from(at.y)?,
        floor: i8::try_from(at.floor)?,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position,
            item: item.into(),
            count,
        },
        n,
        true,
    )
    .await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC2_ALBINIUS_PORTALS_POSTGRES_SERVER_VERSION={version} case={name}");
    if name.ends_with("-ice") {
        return Box::pin(exercise_ice(owner, admitted, h, authority)).await;
    }
    let combined = name.ends_with("combined");
    give(owner, admitted, "oteryn:item.tibia.i23986", 5, 9200).await?;
    for (index, (_, _, item, _)) in PORTALS.iter().enumerate() {
        give(
            owner,
            admitted,
            item,
            if combined { 50 } else { 49 },
            9201 + u64::try_from(index)?,
        )
        .await?;
    }
    let [npc] = install(owner, admitted, ["oteryn:npc.albinius"]).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    talk(owner, admitted, npc, 35000, "hi").await?;
    let premature = talk(owner, admitted, npc, 35001, "holy portal yes").await?;
    assert!(
        premature
            .iter()
            .any(|s| s == "Sorry, first you need to bring my Heavy Old Tomes."),
        "missing source prerequisite refusal: {premature:?}"
    );
    for (_, track, item, _) in PORTALS {
        assert_eq!(
            progress(owner, admitted, authority, &format!("{PREFIX}{track}")).await?,
            0
        );
        assert_eq!(carried(h, item).await?, if combined { 50 } else { 49 });
    }
    let paid = talk(owner, admitted, npc, 35002, "temple yes").await?;
    assert!(
        paid.iter()
            .any(|s| s.contains("Your first step in the ways of the {Shapers}"))
    );
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}tomes")).await?,
        1
    );
    assert_eq!(carried(h, "oteryn:item.tibia.i23986").await?, 0);
    if combined {
        let result = talk(
            owner,
            admitted,
            npc,
            35003,
            "holy portal energy portal earth portal death portal yes",
        )
        .await?;
        assert_eq!(
            result.len(),
            9,
            "retained tome topic plus four independent offers and acceptances: {result:?}"
        );
        assert!(result[0].starts_with("You already offered enough tomes"));
    } else {
        for (index, (keyword, track, item, kind)) in PORTALS.iter().enumerate() {
            let base = 35100 + u64::try_from(index)? * 20;
            let offered = talk(owner, admitted, npc, base, keyword).await?;
            assert!(
                offered.iter().any(|s| s.contains("50") && s.contains(kind)),
                "missing source portal offer: {offered:?}"
            );
            let declined = talk(owner, admitted, npc, base + 1, "no").await?;
            assert!(
                declined
                    .iter()
                    .any(|s| s == "In this case I'm sorry, you may not pass this portal.")
            );
            // Source no leaves the portal topic open; yes still checks exact payment.
            let short = talk(owner, admitted, npc, base + 2, "yes").await?;
            assert!(
                short
                    .iter()
                    .any(|s| s.starts_with("I'm sorry, you don't have enough")),
                "49 items cannot pay: {short:?}"
            );
            assert_eq!(carried(h, item).await?, 49);
            assert_eq!(
                progress(owner, admitted, authority, &format!("{PREFIX}{track}")).await?,
                0
            );
            talk(owner, admitted, npc, base + 3, "bye").await?;
            talk(owner, admitted, npc, base + 4, "hi").await?;
            give(owner, admitted, item, 1, 9300 + u64::try_from(index)?).await?;
            super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, base + 5).await?;
            assert_eq!(carried(h, item).await?, 50);
            let accepted = talk(owner, admitted, npc, base + 6, &format!("{keyword} yes")).await?;
            assert_eq!(
                accepted.len(),
                2,
                "independent source offer+accept: {accepted:?}"
            );
            assert!(
                accepted
                    .iter()
                    .any(|s| s.starts_with("Thank you for your offering."))
            );
            assert_eq!(carried(h, item).await?, 0);
            assert_eq!(
                progress(owner, admitted, authority, &format!("{PREFIX}{track}")).await?,
                1
            );
            let repeated = talk(owner, admitted, npc, base + 7, "yes").await?;
            assert!(
                repeated
                    .iter()
                    .any(|s| s.starts_with("I'm sorry, you don't have enough"))
            );
        }
    }
    for (_, track, item, _) in PORTALS {
        assert_eq!(
            progress(owner, admitted, authority, &format!("{PREFIX}{track}")).await?,
            1
        );
        assert_eq!(carried(h, item).await?, 0);
    }
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}access_ice")).await?,
        0,
        "other quest prerequisite was never earned"
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after - before,
        if combined { 2 } else { 5 },
        "one receipt per committed command, none for missing/stale/repeated payments"
    );
    Ok(())
}

async fn exercise_ice(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [npc] = install(owner, admitted, ["oteryn:npc.albinius"]).await?;
    give(owner, admitted, FISH, 51, 9400).await?;
    let (_, old_hash) = ICE_REVISION.get().ok_or("fixture revision")?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    assert_ne!(catalogue.definition_hash(QUEST).as_ref(), Some(old_hash));

    // Rehydrate a character who paid the tomes under the previous programme,
    // and whose separate Contactman prerequisite has already been earned.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let character = id(41);
    sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision)
         SELECT character_id,$2,$3,$4,character_revision,character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice()).bind(QUEST).bind(catalogue.content_revision()).bind(old_hash.as_slice())
    .execute(&mut *tx).await?;
    for (quest, track) in [
        (QUEST, format!("{PREFIX}tomes")),
        ("oteryn:quest.the_ice_islands_quest", CONTACTMAN.into()),
    ] {
        sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision)
             SELECT character_id,$2,$3,1,character_revision
             FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_slice()).bind(quest).bind(track)
        .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    talk(owner, admitted, npc, 35301, "hi").await?;
    let offer = talk(owner, admitted, npc, 35302, "ice portal").await?;
    assert!(
        offer.iter().any(|line| line.contains("50 fish")),
        "{offer:?}"
    );
    let accepted = talk(owner, admitted, npc, 35303, "yes").await?;
    assert!(
        accepted
            .iter()
            .any(|line| line.contains("Portal to the Powers of Ice")),
        "{accepted:?}"
    );
    assert_eq!(carried(h, FISH).await?, 1);
    assert_eq!(progress(owner, admitted, authority, ICE).await?, 1);
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}tomes")).await?,
        1
    );
    talk(owner, admitted, npc, 35304, "yes").await?;
    assert_eq!(carried(h, FISH).await?, 1);
    let stored_hash: Vec<u8> = sqlx::query_scalar(
        "SELECT definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
    ).bind(character.as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(stored_hash, old_hash.as_slice());
    Ok(())
}
