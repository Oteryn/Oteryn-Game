//! Inspect visible items through capability 39. Only confirmed values are drawn.
use super::*;
use oteryn_client_runtime::timed_item::{TimedDisposition, TimedItemResult};
use oteryn_session::ItemHandle;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Component)]
pub(super) struct Expiry;
#[derive(Default)]
pub(super) struct Samples {
    generation: Option<u64>,
    checked: BTreeMap<ItemHandle, (Instant, Option<TimedItemResult>)>,
    last: Option<Instant>,
}
fn label(reply: TimedItemResult, unused: bool) -> Option<String> {
    if reply.retired
        || reply.disposition != TimedDisposition::Ok
        || (reply.revision == 0 && !unused)
    {
        return None;
    }
    if let Some(ms) = reply.remaining_ms {
        let seconds = ms.div_ceil(1000);
        return Some(format!("{}m {}s", seconds / 60, seconds % 60));
    }
    reply.charges.map(|n| format!("{n}×"))
}
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    slots: Query<(Entity, &Slot)>,
    mut labels: Query<(&ChildOf, Entity, &mut Text), With<Expiry>>,
    mut samples: Local<Samples>,
) {
    let Some(play) = &owner.play else {
        *samples = Samples::default();
        return;
    };
    let prefs = &owner.settings.preferences;
    if samples.generation != Some(owner.generation()) {
        *samples = Samples {
            generation: Some(owner.generation()),
            ..default()
        };
    }
    if let Some((handle, result)) = play.link.take_slot_expiry()
        && let Some(entry) = samples.checked.get_mut(&handle)
    {
        entry.1 = result;
    }
    if !prefs.expiry_inventory
        && !prefs.expiry_containers
        && samples.checked.is_empty()
        && labels.is_empty()
    {
        return;
    }
    let mut visible = BTreeMap::new();
    for (entity, slot) in &slots {
        let enabled = match slot.location {
            Location::Equipment(_) => prefs.expiry_inventory,
            Location::Inventory | Location::Container { .. } => prefs.expiry_containers,
        };
        if enabled
            && slot.generation == owner.generation()
            && let Some(item) = slot.item
        {
            visible.insert(entity, item.handle);
        }
    }
    let handles: std::collections::BTreeSet<_> = visible.values().copied().collect();
    samples.checked.retain(|handle, _| handles.contains(handle));
    let now = Instant::now();
    if samples
        .last
        .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
        && let Some(handle) = handles
            .into_iter()
            .filter(|h| {
                samples
                    .checked
                    .get(h)
                    .is_none_or(|(t, _)| now.duration_since(*t) >= Duration::from_secs(60))
            })
            .min_by_key(|h| samples.checked.get(h).map(|v| v.0))
        && play.link.inspect_slot_expiry(handle)
    {
        samples.checked.entry(handle).or_insert((now, None)).0 = now;
        samples.last = Some(now);
    }
    let text_for = |h: &ItemHandle| {
        samples
            .checked
            .get(h)
            .and_then(|(_, v)| *v)
            .and_then(|v| label(v, prefs.expiry_unused))
    };
    for (parent, entity, mut text) in &mut labels {
        if let Some(value) = visible.remove(&parent.parent()).as_ref().and_then(text_for) {
            if text.0 != value {
                text.0 = value;
            }
        } else {
            commands.entity(entity).try_despawn();
        }
    }
    for (slot, handle) in visible {
        if let Some(value) = text_for(&handle) {
            commands.spawn((
                Expiry,
                oteryn_client_ui_bevy::text(
                    value,
                    bevy::text::TextFont {
                        font_size: bevy::text::FontSize::Px(9.),
                        ..oteryn_client_ui_bevy::font(skins.font.clone(), false)
                    },
                ),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: px(0.),
                    right: px(0.),
                    ..default()
                },
                ChildOf(slot),
            ));
        }
    }
}
