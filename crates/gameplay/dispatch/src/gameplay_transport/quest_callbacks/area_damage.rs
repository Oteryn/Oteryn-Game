//! Source MoveEvents that apply ordinary typed combat damage.
use super::*;
use crate::content::ReferenceResistanceKind;
use crate::gameplay_transport::paced_movement::special_movement::{TileEntries, tile_entries};
use crate::player_lethal::PlayerHitKind;
use oteryn_protocol_oteryn::analyser::AnalyserDamageSource;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_area_damage(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        entered: MovementPositionSnapshot,
    ) {
        let Some(entries) = tile_entries(map, entered.position()) else {
            return;
        };
        self.step_five_ways(actor, session, map, entered, &entries)
            .await;
        self.step_demon_oak_area(actor, session, map, entered, &entries)
            .await;
    }

    /// Crystal's Five Ways fire tiles use the base vocation, including promotions.
    async fn step_five_ways(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        entered: MovementPositionSnapshot,
        entries: &TileEntries,
    ) {
        let mut fires = entries
            .entries
            .iter()
            .filter(|(_, entry)| entry.bound)
            .filter_map(|(key, _)| {
                (2040..=2069).find(|action| {
                    map.facts
                        .quest_callback_matches(*key, None, Some(*action), None)
                })
            });
        let Some(action) = fires.next() else {
            return;
        };
        if fires.next().is_some() {
            return;
        }
        let runtime = self.runtime.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered) {
            return;
        }
        let mut states = self.states.spell_states.lock().await;
        let Some(state) = states.get(&runtime, actor, session) else {
            return;
        };
        use crate::content::ReferenceBaseVocation::{Druid, Knight, Monk, Paladin, Sorcerer};
        let index = usize::from(action - 2040);
        let matching =
            crate::gameplay_transport::item_use::base_vocation(state.character_facts().vocation)
                == [Sorcerer, Druid, Paladin, Knight, Monk][index / 6];
        let column = index % 6;
        let (raw, kind, element) = if matching || column == 0 {
            (300, PlayerHitKind::Fire, ReferenceResistanceKind::Fire)
        } else {
            (
                [300, 600, 1200, 2400, 3600, 7200][column],
                PlayerHitKind::Physical,
                ReferenceResistanceKind::Physical,
            )
        };
        if let Some(position) = native(entered.position())
            && environmental_hit(
                &runtime,
                &mut states,
                actor,
                session,
                raw,
                kind,
                element,
                self.owner_now(),
            )
        {
            map.facts
                .emit_movement_effect(position, if matching { 16 } else { 37 });
        }
    }

    async fn step_demon_oak_area(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        entered: MovementPositionSnapshot,
        entries: &TileEntries,
    ) {
        let at = entered.position();
        let Some(active) = self.active_generation else {
            return;
        };
        let Some(ground) = active
            .native_gameplay()
            .and_then(|native| native.item_index())
            .and_then(|index| {
                index.definition_ref("Item", "oteryn:item.tibia.i918", "definition-r1")
            })
        else {
            return;
        };
        if !entries.entries.iter().any(|(_, entry)| {
            entry.definition == oteryn_protocol_oteryn::world_map::MapDefinition::Item(ground)
        }) {
            return;
        }
        let runtime = self.runtime.lock().await;
        if runtime.content_pin().server_artifact_digest()
            != active.identity().server_artifact_digest()
            || runtime.read_actor_position(actor) != Ok(entered)
        {
            return;
        }
        let draw = super::demon_oak::oak_draw(
            runtime.content_pin().server_artifact_digest(),
            actor,
            session,
            entered.revision(),
            "demon_oak.area",
            0,
        );
        let Some(draw) = draw else { return };
        if crate::spell::uniform_draw(draw, 1, 24) != 1 {
            return;
        }
        let mut states = self.states.spell_states.lock().await;
        if let Some(position) = native(at) {
            oak_earth_hit(
                &runtime,
                &mut states,
                actor,
                session,
                map,
                position,
                (270, 310),
                super::demon_oak::oak_draw(
                    runtime.content_pin().server_artifact_digest(),
                    actor,
                    session,
                    entered.revision(),
                    "demon_oak.area",
                    1,
                ),
                self.owner_now(),
            );
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "passes the active actor, proven source and typed damage to the HP owner"
)]
pub(super) fn oak_earth_hit(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    map: &BundleMap,
    position: TilePos,
    (minimum, maximum): (u32, u32),
    draw: Option<u64>,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) {
    let Some(raw) = draw.and_then(|draw| {
        oteryn_gameplay_domain::domain::source_normal_draw::source_normal_draw(
            draw, minimum, maximum,
        )
    }) else {
        return;
    };
    if environmental_hit(
        runtime,
        states,
        actor,
        session,
        i64::from(raw),
        PlayerHitKind::Earth,
        ReferenceResistanceKind::Earth,
        now,
    ) {
        map.facts.emit_movement_effect(position, 51);
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "typed environmental hit retains the active equipment and HP owners"
)]
fn environmental_hit(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    raw: i64,
    kind: PlayerHitKind,
    element: ReferenceResistanceKind,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> bool {
    let Some(state) = states.get(runtime, actor, session) else {
        return false;
    };
    let damage = state
        .equipped_element_protection(element)
        .and_then(|protection| {
            oteryn_gameplay_items::equipment_effects::apply_protection(raw, protection)
        })
        .unwrap_or(raw);
    let Ok(damage) = u32::try_from(damage) else {
        return false;
    };
    states
        .apply_creature_damage_kind(
            runtime,
            actor,
            session,
            damage,
            kind,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
            AnalyserDamageSource::None,
        )
        .is_some()
}
