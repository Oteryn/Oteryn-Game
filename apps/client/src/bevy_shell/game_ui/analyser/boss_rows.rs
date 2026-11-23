//! Each cooldown links to its boss catalogue entry.
use super::*;
use crate::bevy_shell::ui::{Control as Toolbar, toolbar};
use oteryn_client_ui::boss_encounter::{BossCategory, Intent};
use oteryn_client_ui_bevy as source;

#[derive(Component)]
struct BossLink(u64, String);

pub(super) fn replace(
    commands: &mut Commands,
    root: Entity,
    skins: &Skins,
    generation: u64,
    keys: &[String],
) {
    commands.entity(root).despawn_children();
    for key in keys {
        let row = commands
            .spawn((
                source::button(skins.buttons.clone(), 240.),
                BossLink(generation, key.clone()),
            ))
            .observe(open)
            .id();
        let caption = commands
            .spawn(source::button_caption(
                &format!("Bosstiary: {key}"),
                skins.font.clone(),
            ))
            .id();
        commands.entity(row).add_child(caption);
        commands.entity(root).add_child(row);
    }
}
fn open(
    event: On<Activate>,
    links: Query<&BossLink>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
) {
    let Ok(BossLink(generation, key)) = links.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || owner.modal_active() || owner.generation() != *generation {
        return;
    }
    let Some(play) = &owner.play else { return };
    let bosses = play.link.boss_encounter_panel();
    let panel = play.link.analyser_panel();
    if bosses.pending
        || !bosses.selected
        || !panel.open_tools.contains(&Tool::Bosses)
        || !oteryn_client_ui::analyser::bosses::rows(&panel, &bosses)
            .iter()
            .any(|(name, _)| name == key)
    {
        return;
    }
    play.link
        .boss_encounter_browse(key, BossCategory::All, false);
    if owner.settings.preferences.panel_visible("bosstiary") {
        play.link.boss_encounter_request(Intent::Catalogue);
    } else {
        toolbar::open_toolbar(&mut owner, Toolbar::Panel("bosstiary"));
    }
}
