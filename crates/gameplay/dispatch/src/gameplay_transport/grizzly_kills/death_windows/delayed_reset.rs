use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use crate::map::{
    boot::BundleMap,
    facts::QuestCallbackProjection,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_entities::carrier::native_source_death::SourcePlayerDamageShare;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, DeathMapReset};

#[derive(Debug)]
pub(super) struct PendingReset {
    callback: String,
    rule: DeathMapReset,
    death: ExactActorRef,
    damagers: Vec<SourcePlayerDamageShare>,
    pub(super) due_ms: u64,
}

impl PendingReset {
    // Crystal 201189f8 creaturescripts_bosses_kill.lua:136-144 schedules one
    // reset for each direct damaging player who is still present at expiry.
    pub(super) fn capture(
        runtime: &ChannelRuntimeV1,
        death: ExactActorRef,
        key: &str,
        rule: &DeathMapReset,
        death_ms: u64,
    ) -> Option<Self> {
        let mut damagers = runtime.source_player_damage_shares(death).ok()?;
        damagers.retain(|hit| hit.total > 0 && live(runtime, hit));
        (!damagers.is_empty()).then(|| Self {
            callback: key.to_owned(),
            rule: rule.clone(),
            death,
            damagers,
            due_ms: death_ms.saturating_add(u64::from(rule.delay_ms)),
        })
    }

    pub(super) fn matches(&self, callback: &Callback) -> bool {
        self.callback == callback.key
            && callback
                .effects
                .death_window
                .as_ref()
                .and_then(|rule| rule.delayed_reset.as_ref())
                == Some(&self.rule)
    }

    /// Return whether the pending map write should be retried.
    pub(super) fn apply_if_live(&self, runtime: &ChannelRuntimeV1, map: &BundleMap) -> bool {
        let scope = runtime.binding();
        self.death.runtime_scope() == scope.runtime_scope()
            && self.death.scope_generation() == scope.scope_generation()
            && self.damagers.iter().any(|hit| live(runtime, hit))
            && !project(map, &self.rule)
    }
}

fn live(runtime: &ChannelRuntimeV1, hit: &SourcePlayerDamageShare) -> bool {
    runtime
        .bound_attacker_lease(hit.actor, hit.session)
        .is_ok_and(|lease| {
            lease.character_id() == hit.character && lease.generation() == hit.lease_generation
        })
}

fn project(map: &BundleMap, rule: &DeathMapReset) -> bool {
    // As in the donor, absent or already inactive crystals require no write.
    // The remaining active objects share one version-checked map projection.
    let mut changes = rule.positions.iter().filter_map(|position| {
        let pos = TilePos {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let mut candidates = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .enumerate()
            .filter_map(|(ordinal, (id, _))| {
                let ordinal = u8::try_from(ordinal).ok()?;
                let key = placement_key(pos, ordinal)?;
                let current = map.facts.base_entry(pos, ordinal, *id)?;
                (!map.facts.base_hidden(key)
                    && !current.pickupable
                    && current.appearance_id == rule.from)
                    .then_some(key)
            });
        let key = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        let after = map.facts.quest_callback_replacement(key, rule.to)?;
        (!after.pickupable).then(|| (key, map.facts.object_revision(key), after))
    });
    let Some((key, revision, after)) = changes.next() else {
        return true;
    };
    let entries = changes
        .map(|(key, revision, after)| (key, revision, Some(after)))
        .collect();
    map.facts.apply_quest_callback(
        key,
        revision,
        after,
        &[],
        &[],
        false,
        Some(&QuestCallbackProjection::ReadsetTransforms {
            entries,
            reset: None,
        }),
    )
}
