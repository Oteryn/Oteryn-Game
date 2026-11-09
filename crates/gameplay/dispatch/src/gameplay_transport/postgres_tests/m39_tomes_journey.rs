//! M39a: twelve physical source Tomes, incremental NPC topics and exact hand-ins.
//! Crystal9f5a72c cael.lua:84-133 sha256:4d4d0c4aa36722ebb85925c685b7631471889dd1f51e641711cee51568336dcd.
use super::fixture::{Harness, debug, id};
use super::m39_bone_journey::{carried, grant, stage, use_at};
use super::quest_world_callbacks::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::{
    npc_service::NpcIntentDisposition,
    world_object::UseDisposition,
    world_spatial::{StepDirection, StepDisposition},
};
#[path = "m39_tomes_journey/extra.rs"]
mod extra;
#[path = "m39_tomes_journey/prerequisites.rs"]
mod prerequisites;
#[path = "m39_tomes_journey/world.rs"]
pub(crate) mod world;
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/";
const COUNTER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge";

#[test]
fn m39a_tomes_twelve_source_books_and_incremental_handins() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-tomes")
}

#[test]
fn m39b_tomes_repeatable_paid_xp_and_no_free_reward() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-tomes-extra")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_PLAYER_JOURNEY m39a-tomes server_version={version}");
    assert_eq!(stage(h, COUNTER).await?, None);
    assert_eq!(carried(h, 10217).await?, 0, "no seeded books");
    prerequisites::install(owner, admitted).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 13, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Tomes capabilities")?;
    let [cael, curos, zalamon] = super::classic_quests::install_above(
        owner,
        admitted,
        ["oteryn:npc.cael", "oteryn:npc.curos", "oteryn:npc.zalamon"],
    )
    .await?;
    for (n, key) in ["frontier-ready", "curos-ready", "zalamon-ready"]
        .into_iter()
        .enumerate()
    {
        prerequisites::apply(owner, admitted, key, 18_000 + u64::try_from(n)?).await?;
    }
    assert_eq!(
        grant(owner, admitted, cael, "l2-cael-tome-1").await?.0,
        NpcIntentDisposition::Rejected,
        "no free counter progress"
    );
    assert_eq!(stage(h, COUNTER).await?, None);
    for (npc, key) in [(curos, "l2-alliance-finish"), (zalamon, "l2-zalamon-tome")] {
        assert_eq!(
            grant(owner, admitted, npc, key).await?.0,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(carried(h, 10217).await?, 2);
    let mut books = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.starts_with("oteryn:quest-callback/l2-tome-"))
        .map(|c| c.selector.position.ok_or("book position"))
        .collect::<Result<Vec<_>, _>>()?;
    books.sort_by_key(|p| (p.floor, p.x, p.y));
    assert_eq!(books.len(), 10);
    for p in books {
        let (outcome, command, payload, result) = use_at(owner, admitted, p).await?;
        assert_eq!(outcome, UseDisposition::Committed);
        assert_eq!(
            wire(owner, admitted, command, 2, payload).await?,
            result,
            "literal chest replay"
        );
        assert_eq!(
            use_at(owner, admitted, p).await?.0,
            UseDisposition::Rejected,
            "a new claim cannot duplicate a source book"
        );
        let command = admitted.continuity.next_command_id;
        let result = wire(
            owner,
            admitted,
            command,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::South),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result_paced(&result, true)
                .map_err(debug)?,
            StepDisposition::Moved
        );
    }
    assert_eq!(
        carried(h, 10217).await?,
        12,
        "two NPC gifts and ten real chest claims"
    );
    for n in 1..=12 {
        if n == 7 {
            refuse_busy_offer(owner, admitted, cael).await?;
            assert_eq!(carried(h, 10217).await?, 6);
            prerequisites::apply(owner, admitted, "frontier-finished", 18_005).await?;
        }
        let key = format!("l2-cael-tome-{n}");
        let (result, command) = grant(owner, admitted, cael, &key).await?;
        assert_eq!(result, NpcIntentDisposition::Ok, "Tome {n}");
        assert_eq!(stage(h, COUNTER).await?, Some(n));
        assert_eq!(carried(h, 10217).await?, 12 - n);
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
            &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: cael,
                text: "yes".into(),
            },
        )
        .map_err(debug)?;
        let replay = wire(owner, admitted, command, 7, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
                .map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(stage(h, COUNTER).await?, Some(n));
        assert_eq!(carried(h, 10217).await?, 12 - n);
    }
    for flag in ["snake_head_teleport", "corruption_hole", "zao_palace_doors"] {
        assert_eq!(stage(h, &format!("{FRONTIER}{flag}")).await?, Some(1));
    }
    if name.ends_with("-extra") {
        extra::exercise(owner, admitted, h, cael).await?;
    }
    Ok(())
}

async fn refuse_busy_offer(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/l2-cael-tome-7"))
        .ok_or("seventh Tome")?;
    // A native disconnect ends NPC focus. Keep greeting and refusal together,
    // exactly as the successful hand-in conversations above do.
    let (result, _, _) = super::qengine_rewards::wire_conversation(
        owner,
        admitted,
        npc,
        &callback,
        false,
        vec!["hi", "tome"],
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Rejected);
    Ok(())
}
