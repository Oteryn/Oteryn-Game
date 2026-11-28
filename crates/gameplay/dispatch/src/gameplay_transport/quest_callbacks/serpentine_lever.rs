//! The Crystal action owns the shared scene. The authored opening records personal progress.
use super::*;

pub(super) const SOURCE: &str = "oteryn:quest-callback/serpentine-tower-lever";
pub(super) const JOURNEY: &str = "oteryn:quest-callback/authored-serpentine-tower-lever";

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn serpentine_journey_opening(
        &self,
        source: &Callback,
        map: &BundleMap,
        target: &super::super::world_map::MapObjectTarget<()>,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<&Callback> {
        if source.key != SOURCE {
            return None;
        }
        let journey = self.callback_content().iter().find(|c| c.key == JOURNEY)?;
        let base = map
            .overlay
            .base()
            .tile(target.pos.x, target.pos.y, target.pos.floor)?;
        let (id, _) = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(target.ordinal))?;
        let entry = map.facts.base_entry(target.pos, target.ordinal, *id)?;
        self.serpentine_opening_source(journey, map, target.placement_key, entry.appearance_id)?;
        let tracks = self
            .states
            .quest_sessions
            .lock()
            .ok()?
            .get(&session)?
            .copy
            .as_ref()?
            .read_tracks(self.quest_catalogue.as_deref()?)?;
        let actor_at = self.runtime.lock().await.read_actor_position(actor).ok()?;
        // Selection is advisory. The durable Quest owner rechecks stage and Premium.
        if unique_eligible([journey], &tracks, Some(actor_at.position())).is_none()
            || !self.session_premium_gate(session).await.premium_current()
        {
            return None;
        }
        Some(journey)
    }

    pub(super) fn serpentine_opening_source(
        &self,
        journey: &Callback,
        map: &BundleMap,
        key: u64,
        appearance: u16,
    ) -> Option<&Callback> {
        let (pos, ordinal) = crate::map::view::placement_of(key)?;
        if journey.selector.position.map(tile) != Some(pos)
            || journey.selector.ordinal != Some(u16::from(ordinal))
            || journey.lever?[0] != appearance
        {
            return None;
        }
        self.callback_content().iter().find(|source| {
            source.key == SOURCE
                && source.selector.action == Some(5633)
                && source.lever == journey.lever
                && source.effects.scene.is_some()
                && matches(map, source, key)
        })
    }
}
