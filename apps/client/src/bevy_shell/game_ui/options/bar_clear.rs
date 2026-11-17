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
    owner: NonSend<HostOwner>,
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
    for slot in row * ROW_SLOTS..(row + 1) * ROW_SLOTS {
        play.link.clear_bar_action(slot);
        play.link.assign_spell_slot(slot, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bevy_shell::owner::ActivePlay;
    use crate::play::{BarAction, PlayView, play_channel};
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn clear_buttons_empty_only_the_chosen_bar_of_each_side() -> TestResult {
        for (side, rows) in SIDES {
            let mut app = super::super::tests::app(Page::ActionBars, None)?;
            let (link, _, _) = play_channel();
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
                owner.play = Some(ActivePlay { view, link });
            }
            let target = rows[1];
            let button = app.world_mut().spawn(ClearBar(target)).observe(clear).id();
            app.world_mut().trigger(Activate { entity: button });
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
            assert_eq!(left, expected, "{side} Bar 2 clears only row {target}");
        }
        Ok(())
    }
}
