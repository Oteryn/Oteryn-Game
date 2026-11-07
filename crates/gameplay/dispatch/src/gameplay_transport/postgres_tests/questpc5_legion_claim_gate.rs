//! Component proof of reusable claim predicates; seeded contacts do not qualify the route.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

const CLAIM: &str = "oteryn:reward-claim.kv.legiondawnport";
#[test]
fn questpc5_legion_helmet_claim_requires_both_personal_contacts_and_is_once_only() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-newhaven-legion-gates")
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let chest = super::quest_catalogue_chests::CatalogueChest {
        claim: CLAIM.into(),
        position: (32143, 31910, -8),
        unique: 6006,
        servable: true,
        appearance: 4240,
        root: "oteryn:item.tibia.i3374".into(),
        quantity: 1,
        contents: Vec::new(),
        texts: Vec::new(),
        key_number: None,
        achievement: false,
        charges: Vec::new(),
        fluids: Vec::new(),
    };
    super::quest_catalogue_chests::catalogue_source_world(&chest)
}
async fn use_corpse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    use_corpse_at_ordinal(owner, admitted, 1).await
}
pub(super) async fn use_corpse_at_ordinal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ordinal: u8,
) -> TestResult<oteryn_protocol_oteryn::world_object::UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key =
        oteryn_world_bundle::bundle::placement_key(-8, 32143, 31910, ordinal).ok_or("corpse")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = oteryn_protocol_oteryn::world_object::WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
    )
    .await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}
pub(super) async fn count(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3374' AND lifecycle=1)").bind(id(41).as_slice()).bind(CLAIM).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    };
    let started = std::time::Instant::now();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    super::quest_catalogue_chests::relocate_source_actor(
        owner,
        admitted,
        admitted.runtime_actor.ok_or("actor")?,
        Position {
            x: 32142,
            y: 31910,
            floor: -8,
        },
    )
    .await?;
    assert_eq!(count(h).await?, (0, 0));
    assert_ne!(
        use_corpse(owner, admitted).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
        "helmet must refuse without both personal route contacts"
    );
    assert_eq!(
        count(h).await?,
        (0, 0),
        "refusal has no physical reward or claim"
    );
    let original = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let mut tracks = Vec::new();
    let mut transitions = Vec::new();
    for name in ["gustavo", "viola", "cave"] {
        let track = format!("oteryn:quest-progress/legion-helmet/{name}");
        tracks.push(original.track(&track).cloned().ok_or("contact track")?);
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/fixture-legion-{name}"),
            quest: "oteryn:quest.newhaven_legion_helmet_quest".into(),
            effects: vec![QuestEffect {
                track,
                from: QuestComparison::Eq(-1),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
        });
    }
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(original.content_revision(), tracks, transitions)
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
    for (i, name) in ["gustavo", "viola", "cave"].into_iter().enumerate() {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: format!("oteryn:quest-transition/fixture-legion-{name}"),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(9000 + u64::try_from(i)?).map_err(debug)?,
                    )),
                },
                std::sync::Arc::clone(&catalogue),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        if i < 2 {
            assert_ne!(
                use_corpse(owner, admitted).await?,
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
                "both contacts and actual cave predicate are required"
            );
            assert_eq!(count(h).await?, (0, 0));
        }
    }
    assert_eq!(
        use_corpse(owner, admitted).await?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    assert_eq!(count(h).await?, (1, 1));
    let _ = use_corpse(owner, admitted).await?;
    assert_eq!(count(h).await?, (1, 1), "once claim remains authoritative");
    eprintln!(
        "POSTGRES_CASE_COMPLETED legion_claim_gates duration={:?}",
        started.elapsed()
    );
    Ok(())
}
