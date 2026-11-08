//! Four Albinius portal offerings earned after the real five-tome contribution.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

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
