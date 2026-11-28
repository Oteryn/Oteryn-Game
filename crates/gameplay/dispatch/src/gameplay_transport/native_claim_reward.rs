//! A bound authored map claim borrows the fenced quest exchange owner. The
//! registry digest, allocation and callback identity must agree before any write.
use super::ComposedFreshAdmission;
use super::bundle_chest::{item_ref, outcome};
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use crate::content::world_reward_claims::{BoundPlacement, ServedClaim};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_protocol_oteryn::world_object::UseDisposition;

use oteryn_gameplay_quests::quest::QuestStateCatalogue;

const FORGEMASTER_ACCESS: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forge_master/access_door";
const FORGEMASTER_DOOR_STAGE: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";
const FORGEMASTER_CLAIMED: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/reward";
// Crystal 201189f8, no_rest_for_the_wicked_rewards.lua:1-13.
const FORGEMASTER_REWARDS: [(u16, &str); 11] = [
    (57237, "oteryn:item.tibia.i49520"),
    (57238, "oteryn:item.tibia.i49522"),
    (57239, "oteryn:item.tibia.i49523"),
    (57240, "oteryn:item.tibia.i49524"),
    (57241, "oteryn:item.tibia.i49525"),
    (57242, "oteryn:item.tibia.i49526"),
    (57243, "oteryn:item.tibia.i49527"),
    (57244, "oteryn:item.tibia.i49530"),
    (57245, "oteryn:item.tibia.i49528"),
    (57246, "oteryn:item.tibia.i49529"),
    (57247, "oteryn:item.tibia.i50250"),
];

/// Route the Forgemaster's donor selectors through one guarded native claim.
/// The ordinary MINT owner checks the access track and claims the shared key.
pub(super) fn admits_forgemaster_claim(
    map: &crate::map::boot::BundleMap,
    claim: &ServedClaim,
    bound: &BoundPlacement,
    catalogue: Option<&QuestStateCatalogue>,
) -> bool {
    let forgemaster_claim = claim.key.as_str() == "oteryn:reward-claim.nrwicked-forgemaster";
    let matches = |placement: &BoundPlacement, uid| {
        let key = placement.bundle_placement_key;
        if forgemaster_claim && !map.facts.quest_callback_source_bound(key) {
            // The retained World chests have native position/ordinal bindings.
            // Their bound callback names carry the donor selector for the same guard.
            return placement
                .reward_callback
                .as_deref()
                .and_then(|callback| {
                    callback.strip_prefix("oteryn:quest-reward/nrwicked-forgemaster-")
                })
                .and_then(|selector| selector.parse::<u16>().ok())
                == Some(uid);
        }
        map.facts.quest_callback_matches(key, Some(uid), None, None)
    };
    let Some((_, selected_item)) = FORGEMASTER_REWARDS
        .iter()
        .find(|(uid, _)| matches(bound, *uid))
    else {
        return !forgemaster_claim;
    };
    claim.placements.len() == FORGEMASTER_REWARDS.len()
        && FORGEMASTER_REWARDS.iter().all(|(uid, item)| {
            claim.placements.iter().any(|placement| {
                matches(placement, *uid)
                    && matches!(placement.placement.items.as_slice(), [reward]
                        if reward.item.key().as_str() == *item && reward.count == 1)
                    && placement.contents.is_empty()
                    && placement.reward_callback.is_some()
            })
        })
        && callback(catalogue, claim, bound).is_some_and(|reward| {
            // Crystal's key dialogue sets both tracks after the hard-win threshold.
            [(FORGEMASTER_ACCESS, 1), (FORGEMASTER_DOOR_STAGE, 3)]
                .into_iter()
                .all(|(track, value)| {
                    reward.requires.iter().any(|requirement| {
                        requirement.track == track
                            && requirement.minimum == value
                            && requirement.maximum == value
                    })
                })
                && reward.storage.iter().any(|state| {
                    state.track == FORGEMASTER_CLAIMED
                        && state.maximum == 0
                        && state.set == 1
                        && !state.preserve
                        && !state.read_only
                })
                && reward.give.is_some_and(|give| {
                    give.item == *selected_item && give.backpack_outputs.is_empty()
                })
        })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn callback(
    catalogue: Option<&QuestStateCatalogue>,
    claim: &ServedClaim,
    bound: &BoundPlacement,
) -> Option<RewardCallback> {
    let selected = bound.reward_callback.as_deref()?;
    let callback = catalogue
        .map_or_else(reward_callbacks::load, reward_callbacks::load_for)
        .ok()?
        .into_iter()
        .find(|callback| callback.key == selected)?;
    let binding = callback.world_claim.as_ref()?;
    if binding.key != claim.key.as_str()
        || binding.revision != claim.revision.as_str()
        || binding.authority_sha256 != hex(&claim.authority_digest)
        || callback.carried_item.is_some()
    {
        return None;
    }
    let reward = callback.give.as_ref()?;
    let (root, additional) = bound.placement.items.split_first()?;
    if additional.len() != reward.additional.len()
        || reward
            .additional
            .iter()
            .zip(additional)
            .any(|(expected, actual)| {
                expected.item != actual.item.key().as_str()
                    || u32::from(expected.count) != actual.count
                    || expected.fluid_subtype.is_some()
            })
    {
        return None;
    }
    if reward.item != root.item.key().as_str()
        || u32::from(reward.count) != root.count
        || reward.contents.len() != bound.contents.len()
        || reward
            .contents
            .iter()
            .zip(&bound.contents)
            .any(|(expected, actual)| {
                expected.item != actual.item.key().as_str()
                    || u32::from(expected.count) != actual.count
                    || expected.fluid_subtype.is_some()
                    || actual.key_number.is_some()
                    || actual.text.is_some()
            })
        || reward.text.as_ref().and_then(|text| text.root()) != bound.root_text.as_deref()
    {
        return None;
    }
    Some(callback)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn settle_native_claim_premium(
        &self,
        claim: &ServedClaim,
        bound: &BoundPlacement,
        command: UseCommand,
    ) {
        if callback(self.quest_catalogue.as_deref(), claim, bound).is_some_and(|callback| {
            callback.requires_premium || callback.give.is_some_and(|reward| reward.premium)
        }) {
            let _ = self.session_premium_gate(command.game_session_id).await;
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one immutable map claim and the exact live actor/command share the existing physical permit"
    )]
    pub(super) async fn use_native_claim_reward(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        claim: &ServedClaim,
        bound: &BoundPlacement,
        permit: &SpellLanePermit,
    ) -> UseOutcome {
        let Some(callback) = callback(self.quest_catalogue.as_deref(), claim, bound) else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return outcome(UseDisposition::Rejected);
        };
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
            return outcome(UseDisposition::Rejected);
        };
        if command.item_fence != Some(fence.item_fence()) {
            return outcome(UseDisposition::Rejected);
        }
        let context = {
            let runtime = self.runtime.lock().await;
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                return outcome(UseDisposition::Rejected);
            };
            let position = snapshot.position();
            if position.x.abs_diff(i32::from(bound.cell.x)) > 1
                || position.y.abs_diff(i32::from(bound.cell.y)) > 1
                || i32::from(position.floor) != -i32::from(bound.cell.floor)
            {
                return outcome(UseDisposition::TooFar);
            }
            crate::durability::quest_world_callback::QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: snapshot.revision(),
                party: Vec::new(),
            }
        };
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return outcome(UseDisposition::Rejected);
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        // The exact Item facts are resolved by the same admitted generation as
        // the map binder. The quest owner rechecks capacity and costs atomically.
        if bound.placement.items.iter().any(|item| {
            super::item_ref_admission::generation_item_facts(native, &item_ref(&item.item)).is_err()
        }) {
            return outcome(UseDisposition::Rejected);
        }
        self.schedule_quest_refresh(command.game_session_id);
        let result = self
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_world_callback_with_items(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key,
                    cause: QuestCause::Use(CommandRef::new(command.game_session_id, command_id)),
                },
                std::sync::Arc::clone(catalogue),
                permit,
                context,
                Some(native),
                Some(QuestRewardAdmission {
                    selected_fish: None,
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: None,
                    peer_use: None,
                }),
                &self.premium_coordinator,
            )
            .await;
        #[cfg(test)]
        eprintln!("ENGCLAIM_NATIVE_RESULT {result:?}");
        if result.is_ok() {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
        }
        outcome(
            if matches!(
                result,
                Ok(QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_))
            ) {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::world_reward_claims::{RewardClaimBase, RewardClaimTile, UniqueEntry};
    struct Base;
    struct Tile;
    impl RewardClaimBase for Base {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            (x == 32552 && y == 32200 && floor == -11).then_some(Tile)
        }
    }
    impl RewardClaimTile for Tile {
        fn depths(&self) -> &[u8] {
            &[0, 0]
        }
        fn unique(&self, _: u8) -> Option<UniqueEntry> {
            None
        }
        fn appearance(&self, ordinal: u8) -> Option<u16> {
            (ordinal == 1).then_some(4240)
        }
    }
    #[test]
    fn engclaim_m34_allocation_authority_rejects_changed_digest() -> Result<(), String> {
        let report =
            crate::content::world_reward_claims::bind_embedded_reward_claims(&Base, &|_| true)
                .map_err(|e| e.to_string())?;
        let mut claim = report
            .served
            .into_iter()
            .find(|c| c.key.as_str().ends_with("kingdom_of_kormarak.corpse"))
            .ok_or("corpse")?;
        let bound = claim.placements.first().ok_or("placement")?;
        assert_eq!(
            callback(None, &claim, bound)
                .ok_or("exact allocation")?
                .give
                .ok_or("items")?
                .additional
                .len(),
            3
        );
        claim.authority_digest[0] ^= 1;
        assert!(callback(None, &claim, &claim.placements[0]).is_none());
        Ok(())
    }

    #[test]
    fn nrwicked_native_claim_resolves_active_catalogue_callback() -> Result<(), String> {
        let report =
            crate::content::world_reward_claims::bind_embedded_reward_claims(&Base, &|_| true)
                .map_err(|error| error.to_string())?;
        let mut claim = report
            .served
            .into_iter()
            .find(|claim| claim.key.as_str().ends_with("kingdom_of_kormarak.corpse"))
            .ok_or("corpse claim")?;
        let placement = claim.placements.first_mut().ok_or("placement")?;
        let mut authored = reward_callbacks::load()?
            .into_iter()
            .find(|callback| Some(&callback.key) == placement.reward_callback.as_ref())
            .ok_or("source callback")?;
        authored.key = "oteryn:quest-reward/nrwicked-catalogue-fixture".into();
        for storage in &mut authored.storage {
            storage.track.push_str("/nrwicked-catalogue");
        }
        placement.reward_callback = Some(authored.key.clone());
        let tracks = authored
            .storage
            .iter()
            .map(|storage| {
                serde_json::json!({
                    "key": storage.track, "initial": storage.minimum,
                    "minimum": storage.minimum, "maximum": storage.maximum.max(storage.set)
                })
            })
            .chain(authored.requires.iter().map(|required| {
                serde_json::json!({
                    "key": required.track, "initial": required.minimum,
                    "minimum": required.minimum, "maximum": required.maximum,
                })
            }))
            .collect::<Vec<_>>();
        let document = serde_json::json!({
            "schema": 1,
            "quests": [{
                "quest": authored.quest,
                "recipe_sha256": "1".repeat(64),
                "sources": ["native chest catalogue test fixture"],
                "tracks": tracks,
                "stages": [{
                    "key": "claim", "completes": authored.completes,
                    "trigger": {"kind": "npc_dialogue", "callback": authored}
                }]
            }]
        });
        let bindings = oteryn_gameplay_quests::quest::authored_bindings::Bindings::parse(
            &serde_json::to_vec(&document).map_err(|error| error.to_string())?,
        )?;
        let base = QuestStateCatalogue::empty("content-1").ok_or("base catalogue")?;
        let catalogue = bindings.attach(&base)?;
        let placement = claim.placements.first().ok_or("placement")?;
        assert!(callback(None, &claim, placement).is_none());
        let transition = catalogue
            .transition(&authored.key)
            .ok_or("claim transition")?;
        let validate = oteryn_gameplay_quests::quest::admission_binding::validate_for_catalogue;
        validate(&catalogue, transition, true)?;
        assert!(validate(&catalogue, transition, false).is_err());
        assert_eq!(
            callback(Some(&catalogue), &claim, placement),
            Some(authored)
        );
        Ok(())
    }
}
