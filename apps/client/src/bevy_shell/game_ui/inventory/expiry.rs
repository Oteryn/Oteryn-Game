//! Inspect visible items through capability 39. Only confirmed values are drawn.
use super::*;
use oteryn_client_runtime::timed_item::{TimedDisposition, TimedItemResult};
use oteryn_session::ItemHandle;
use std::{collections::BTreeMap, time::Duration};

#[derive(Component)]
pub(super) struct EquipmentCell;
#[derive(Component)]
pub(super) struct Expiry;
struct Sample {
    item: ItemEntry,
    location: Location,
    requested: Duration,
    reply: Option<(Duration, TimedItemResult)>,
}
#[derive(Default)]
pub(super) struct Samples {
    generation: Option<u64>,
    checked: BTreeMap<ItemHandle, Sample>,
    last: Option<Duration>,
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
    clock: Res<Time<Real>>,
    skins: Res<Skins>,
    slots: Query<(Entity, &Slot, Has<EquipmentCell>)>,
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
    let now = clock.elapsed();
    if let Some((handle, result)) = play.link.take_slot_expiry()
        && let Some(entry) = samples.checked.get_mut(&handle)
    {
        entry.reply = result.map(|reply| (now, reply));
    }
    if !prefs.expiry_inventory
        && !prefs.expiry_containers
        && samples.checked.is_empty()
        && labels.is_empty()
    {
        return;
    }
    let mut visible = BTreeMap::new();
    let mut handles = BTreeMap::new();
    for (entity, slot, equipped) in &slots {
        let enabled = if equipped {
            prefs.expiry_inventory
        } else {
            prefs.expiry_containers
        };
        if enabled
            && slot.generation == owner.generation()
            && let Some(item) = slot.item
        {
            visible.insert(entity, item.handle);
            handles.insert(item.handle, (item, slot.location));
        }
    }
    samples
        .checked
        .retain(|handle, sample| handles.get(handle) == Some(&(sample.item, sample.location)));
    if samples
        .last
        .is_none_or(|last| now.saturating_sub(last) >= Duration::from_secs(1))
        && let Some((handle, (item, location))) = handles
            .into_iter()
            .filter(|(h, _)| {
                samples.checked.get(h).is_none_or(|sample| {
                    sample.reply.is_none()
                        && now.saturating_sub(sample.requested) >= Duration::from_secs(60)
                })
            })
            .min_by_key(|(h, _)| samples.checked.get(h).map(|v| v.requested))
        && play.link.inspect_slot_expiry(handle)
    {
        samples.checked.insert(
            handle,
            Sample {
                item,
                location,
                requested: now,
                reply: None,
            },
        );
        samples.last = Some(now);
    }
    let text_for = |h: &ItemHandle| {
        samples
            .checked
            .get(h)
            .and_then(|sample| sample.reply)
            .and_then(|(observed, mut reply)| {
                let elapsed =
                    u64::try_from(now.saturating_sub(observed).as_millis()).unwrap_or(u64::MAX);
                reply.remaining_ms = reply.remaining_ms.map(|ms| ms.saturating_sub(elapsed));
                label(reply, prefs.expiry_unused)
            })
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
