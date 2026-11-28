use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(super) const PHILLIP_SCENARIO: &str = "qengine-rewards-between-the-lines-phillip";

pub(super) const PHILLIP: Position = Position {
    x: 32370,
    y: 31764,
    floor: -7,
};

pub(super) async fn phillip(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC conversation capabilities")?;
    owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    let npc_key = "oteryn:npc.phillip";
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let source = oteryn_world_bundle_compiler::source::project_regions(
        &root,
        &[(PHILLIP.x / 256, PHILLIP.y / 256)],
    )?;
    let contact = source
        .families
        .npc_placements
        .iter()
        .find(|placement| placement.npc == npc_key)
        .ok_or("production Phillip placement")?;
    let (x, y, floor) = contact.cell;
    let position = Position {
        x,
        y,
        floor: -i8::try_from(floor)?,
    };
    assert_eq!(position, PHILLIP);
    let [npc] =
        super::classic_quests_fixtures::install_positions(owner, admitted, [npc_key], [position])
            .await?;
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|row| row.key.ends_with("/between-the-lines-phillip-accept"))
        .ok_or("Phillip's investigation")?;
    let (refused, _, _) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, &callback, false)
            .await?;
    assert_eq!(refused, NpcIntentDisposition::Rejected);
    super::qengine_outfit_stories_fixtures::earn_level(owner, admitted, authority, 200).await?;
    let level: i64 =
        sqlx::query_scalar("SELECT level::bigint FROM game_character_progression_state")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(level, 200);
    let (accepted, _, _) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, &callback, false)
            .await?;
    assert_eq!(accepted, NpcIntentDisposition::Ok);
    let phase: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(&callback.storage[0].track)
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(phase, 1);
    Ok(())
}
