//! Action Bars page: "Clear ... Action Bars: Bar 1 / Bar 2 / Bar 3" buttons per side.
use super::*;
use crate::bevy_shell::game_ui::actions::ROW_SLOTS;
use bevy::ui_widgets::Activate;

/// The bar rows of each side, in the order Tibia numbers them.
pub(super) const SIDES: [(&str, [usize; 3]); 3] = [
    ("Bottom", [0, 3, 4]),
    ("Left", [1, 5, 6]),
    ("Right", [2, 7, 8]),
];

#[derive(Component, Clone, Copy)]
pub(super) struct ClearBar(pub usize);

pub(super) fn build(commands: &mut Commands, parent: Entity, skins: &Skins) {
    for (side, rows) in SIDES {
        section::label(
            commands,
            parent,
            skins,
            &format!("Clear {side} Action Bars"),
        );
        let bar = section::bar(commands, parent);
        for (number, row) in rows.into_iter().enumerate() {
            let button = section::action(
                commands,
                bar,
                skins,
                &format!("Bar {}", number + 1),
                80.,
                &format!("action_bars.clear_{}_{}", side.to_lowercase(), number + 1),
                false,
            );
            commands.entity(button).insert(ClearBar(row)).observe(clear);
        }
    }
}

fn clear(
    event: On<Activate>,
    bars: Query<&ClearBar>,
    mut owner: NonSendMut<HostOwner>,
    state: Res<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::ActionBars) {
        return;
    }
    let (Ok(&ClearBar(row)), Some(play)) = (bars.get(event.entity), owner.play.as_ref()) else {
        return;
    };
    // The same removal as the slot menu's Clear, for every slot of that bar.
    let slots = row * ROW_SLOTS..(row + 1) * ROW_SLOTS;
    for slot in slots.clone() {
        play.link.clear_bar_action(slot);
        play.link.assign_spell_slot(slot, None);
    }
    let owner = &mut *owner;
    if let Some(play) = &owner.play {
        owner.settings.sync_archive_bindings(&play.link);
    }
    let cleared = owner.settings.preferences.archive_bindings.slots[slots.clone()].to_vec();
    owner.edit_preferences(|draft| {
        draft.archive_bindings.slots[slots.clone()].clone_from_slice(&cleared);
        draft.bar_actions.0.retain(|slot, _| !slots.contains(slot));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bevy_shell::owner::ActivePlay;
    use crate::play::{BarAction, PlayView, play_channel};
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn clear_buttons_empty_only_the_chosen_bar_of_each_side() -> TestResult {
        for (side, rows) in [
            ("bottom", [0, 3, 4]),
            ("left", [1, 5, 6]),
            ("right", [2, 7, 8]),
        ] {
            for (number, target) in rows.into_iter().enumerate() {
                let mut app = super::super::tests::app(Page::ActionBars, None)?;
                let (link, _, outbox) = play_channel();
                use oteryn_session::spell_catalog::{SpellCatalogEntry, encode_catalog};
                let catalogue = encode_catalog(&[SpellCatalogEntry {
                    cast_index: 71,
                    name: "Archived healing".into(),
                    words: "exura".into(),
                    level: 1,
                    mana: 0,
                    mana_percent: false,
                    soul: 0,
                    learned: true,
                    available: true,
                    cooldown_groups: vec![],
                    rune_item: None,
                }])
                .map_err(|error| format!("{error:?}"))?;
                crate::play::feature_0287_actor_spell_catalog::protocol::bevy_catalog_fixture(
                    &outbox, catalogue,
                )?;
                let text = |slot: usize| BarAction::Text {
                    text: format!("{side} {slot}"),
                    send_automatically: false,
                };
                let filled: Vec<usize> = (0..9).map(|row| row * ROW_SLOTS + 1).collect();
                for &slot in &filled {
                    assert!(link.assign_bar_action(slot, text(slot)));
                }
                {
                    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
                    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
                    owner.cancel();
                    owner.settings.sync_bar_actions(&link);
                    for row in 0..9 {
                        assert!(link.assign_spell_slot(row * ROW_SLOTS, Some(71)));
                    }
                    owner.settings.sync_archive_bindings(&link);
                    for row in 0..9 {
                        assert!(
                            link.spell_slot_labels()[row * ROW_SLOTS].contains("Archived healing")
                        );
                    }
                    owner.open_graphics();
                    owner.play = Some(ActivePlay { view, link });
                }
                app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
                    .init_asset::<Image>()
                    .init_asset::<Font>()
                    .add_systems(
                        Startup,
                        (
                            crate::bevy_shell::ui::setup,
                            |mut commands: Commands, skins: Res<Skins>| {
                                let root = commands.spawn(Node::default()).id();
                                build(&mut commands, root, &skins);
                            },
                        )
                            .chain(),
                    );
                app.update();
                let name = format!("action_bars.clear_{side}_{}", number + 1);
                let button = app
                    .world_mut()
                    .query::<(Entity, &Name)>()
                    .iter(app.world())
                    .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
                    .ok_or("Clear Bar button")?;
                app.world_mut().trigger(Activate { entity: button });
                app.world_mut().non_send_mut::<HostOwner>().poll([800, 600]);
                let ok = app.world_mut().spawn(Control::Ok).observe(activate).id();
                app.world_mut().trigger(Activate { entity: ok });
                app.world_mut().non_send_mut::<HostOwner>().poll([800, 600]);
                let labels = app
                    .world()
                    .non_send::<HostOwner>()
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .spell_slot_labels();
                let left: Vec<usize> = filled
                    .iter()
                    .copied()
                    .filter(|&slot| labels[slot].contains(&format!("{side} {slot}")))
                    .collect();
                let expected: Vec<usize> = filled
                    .iter()
                    .copied()
                    .filter(|slot| slot / ROW_SLOTS != target)
                    .collect();
                assert_eq!(left, expected, "{side} clears only row {target}");
                for row in 0..9 {
                    assert_eq!(
                        labels[row * ROW_SLOTS].contains("Archived healing"),
                        row != target,
                        "{side} archived spell in row {row}"
                    );
                }
            }
        }
        Ok(())
    }
}
