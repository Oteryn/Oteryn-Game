use super::*;
use std::fmt::Write;

#[derive(Resource, Default)]
pub(super) struct Tracker {
    pub visible: Option<u64>,
}
#[derive(Component)]
pub(super) struct Contents;

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut tracker: ResMut<Tracker>,
    sidebar: Option<Res<super::super::Sidebar>>,
    mut contents: Query<(Entity, &mut Text), With<Contents>>,
) {
    let Some(panel) = owner
        .play
        .as_ref()
        .filter(|_| tracker.visible == Some(owner.generation()))
        .map(|play| play.link.prey_panel())
        .filter(|p| p.selected)
    else {
        tracker.visible = None;
        for (entity, _) in &mut contents {
            commands.entity(entity).despawn();
        }
        return;
    };
    let mut text = String::from("Prey tracker");
    for slot in panel.confirmed.iter().flat_map(|reply| &reply.slots) {
        if slot.state == 2 {
            let _ = write!(
                text,
                "\n{} · {} +{}% · {:02}:{:02}",
                creature_label(&slot.selected),
                super::bonus_name(slot.bonus),
                slot.percent,
                slot.hunting_seconds / 3600,
                slot.hunting_seconds / 60 % 60
            );
        } else {
            let _ = write!(text, "\nSlot {} · {}", slot.slot, slot_title(slot.state));
        }
    }
    if let Ok((_, mut label)) = contents.single_mut() {
        if label.0 != text {
            label.0 = text;
        }
    } else {
        let entity = commands
            .spawn((
                Contents,
                Name::new("prey.tracker"),
                source::text(text, source::font(skins.font.clone(), false)),
                source::frame(skins.inset.clone(), [1.; 4]),
            ))
            .id();
        if let Some(sidebar) = sidebar {
            commands.entity(sidebar.right_widgets).add_child(entity);
        }
    }
}
