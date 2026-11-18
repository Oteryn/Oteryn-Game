//! Crystal 201189f8, creaturescripts_bosses_kill.lua:1-55,126-134.
use super::{ComposedFreshAdmission, PendingKillSettlement};
use crate::map::{
    facts::QuestCallbackProjection,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};

type SourcePosition = (u16, u16, i8);
const PORTALS: [(&str, SourcePosition, SourcePosition); 9] = [
    (
        "oteryn:creature.ferumbras_mortal_shell",
        (33392, 31485, -14),
        (33388, 31414, -14),
    ),
    (
        "oteryn:creature.mazoran",
        (33585, 32699, -14),
        (33614, 32679, -15),
    ),
    (
        "oteryn:creature.plagirath",
        (33174, 31511, -13),
        (33204, 31510, -13),
    ),
    (
        "oteryn:creature.ragiaz",
        (33482, 32345, -13),
        (33466, 32392, -13),
    ),
    (
        "oteryn:creature.razzagorn",
        (33413, 32467, -14),
        (33357, 32440, -13),
    ),
    (
        "oteryn:creature.shulgrax",
        (33486, 32796, -13),
        (33459, 32820, -14),
    ),
    (
        "oteryn:creature.tarbaz",
        (33460, 32853, -11),
        (33427, 32852, -13),
    ),
    (
        "oteryn:creature.the_lord_of_the_lice",
        (33226, 31478, -12),
        (33237, 31477, -13),
    ),
    (
        "oteryn:creature.zamulosh",
        (33644, 32764, -11),
        (33678, 32758, -13),
    ),
];

fn tile((x, y, floor): SourcePosition) -> TilePos {
    TilePos { x, y, floor }
}

pub(super) async fn open(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) {
    let Some((_, source, destination)) = PORTALS.iter().find(|(key, ..)| *key == entry.creature)
    else {
        return;
    };
    let Some(map) = owner.bundle_map() else {
        return;
    };
    let Some(due) = entry
        .facts
        .death_at_ms
        .checked_mul(1_000)
        .and_then(|time| time.checked_add(60_000_000))
    else {
        return;
    };
    let runtime = owner.runtime.lock().await;
    if owner.owner_now().get() >= due
        || runtime.owner_fence().is_err()
        || runtime.binding().runtime_scope() != entry.facts.death.runtime_scope()
        || map.overlay.runtime_scope() != entry.facts.death.runtime_scope()
        || !runtime
            .companion_snapshot_including_dead(entry.target)
            .is_ok_and(|body| {
                body.state.master.is_none()
                    && !body.state.policy.is_familiar
                    && body.state.policy.definition_key == entry.creature
            })
    {
        return;
    }
    let position = tile(*source);
    let destination = tile(*destination);
    let Some(base) = map
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
    else {
        return;
    };
    let mut portals = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (&id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(position, ordinal)?;
            let before = map.facts.base_entry(position, ordinal, id)?;
            (before.appearance_id == 1949 && !before.pickupable && !map.facts.base_hidden(key))
                .then_some((key, before))
        });
    let Some((key, before)) = portals.next() else {
        return;
    };
    if portals.next().is_some() || map.facts.house_tile(position) {
        return;
    }
    let Some(after) = map.facts.quest_callback_replacement(key, 22761) else {
        return;
    };
    if after.pickupable {
        return;
    }
    let projection = QuestCallbackProjection::TimedEntrance {
        due,
        restore: before,
        position,
        destination,
        retreat: Some(tile((33319, 32318, -13))),
    };
    let _ = map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        after,
        &[],
        &[],
        false,
        Some(&projection),
    );
}
