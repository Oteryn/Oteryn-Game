//! Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
//! data-global/npc/an_old_dragon_lord.lua:61-80,
//! sha256:85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052.
//! Each greeting concludes, including insufficient payment and an existing fetish flag.
use super::*;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_CASE_STARTED draconia-greeting server_version={version}");
    let [npc] =
        super::super::classic_quests::install(owner, admitted, ["oteryn:npc.an_old_dragonlord"])
            .await?;
    let before = counts(h).await?;
    let first = greeting(owner, admitted, npc, 1).await?;
    assert!(
        first
            .lines
            .iter()
            .any(|line| line.contains("BRRRING ME MUSHRRROOOMSSS")),
        "unpaid greeting must request mushrooms: {first:?}"
    );
    assert!(first.open.is_none(), "refusal must release NPC focus");
    assert_eq!(counts(h).await?, before, "refusal cannot write a receipt");
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    super::super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i3723".into(),
            count: 2,
        },
        700,
        true,
    )
    .await?;
    let paid = greeting(owner, admitted, npc, 2).await?;
    assert!(
        paid.lines
            .iter()
            .any(|line| line.contains("TAKE THISS AND LEAVE")),
        "{paid:?}"
    );
    assert!(
        paid.open.is_none(),
        "successful exchange must release focus"
    );
    let after = counts(h).await?;
    assert_eq!(after.0, before.0 + 1, "one atomic quest receipt");
    for (item, expected) in [
        ("oteryn:item.tibia.i3723", 1_i64),
        ("oteryn:item.tibia.i3206", 1),
    ] {
        let quantity:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(item).fetch_one(&h.pool).await?;
        assert_eq!(quantity, expected, "{item}");
    }
    let repeated = greeting(owner, admitted, npc, 3).await?;
    assert!(
        repeated
            .lines
            .iter()
            .any(|line| line.contains("LEAVE THE DRAGONS' CEMETERY AT ONCE!")),
        "{repeated:?}"
    );
    assert!(repeated.open.is_none());
    assert_eq!(
        counts(h).await?,
        after,
        "an existing flag cannot charge again"
    );
    Ok(())
}

async fn greeting(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command_id: u64,
) -> TestResult<oteryn_protocol_oteryn::npc_service::NpcConversation> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "hi".into(),
            },
        )
        .await;
    assert!(!outcome.unknown);
    outcome.conversation.ok_or_else(|| "greeting reply".into())
}
