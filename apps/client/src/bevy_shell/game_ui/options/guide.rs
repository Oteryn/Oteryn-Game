//! The Misc page's local help guide: choose a topic and read it in the options window.
use super::*;
use crate::play::help::Topic;
use bevy::ui_widgets::Activate;

#[derive(Component, Clone, Copy)]
pub(super) struct Pick(Topic);

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    help: Option<&(Topic, String)>,
) {
    let selected = help.map(|(topic, _)| *topic);
    section::label(commands, parent, skins, tr!("Help"));
    let bar = section::bar(commands, parent);
    for topic in Topic::ALL {
        let button = section::action(
            commands,
            bar,
            skins,
            topic.label(),
            150.,
            &format!("misc.help.{topic:?}"),
            selected.is_some_and(|chosen| chosen == topic),
        );
        commands.entity(button).insert(Pick(topic)).observe(pick);
    }
    if let Some((_, text)) = help {
        section::label(commands, parent, skins, text);
    }
}

fn pick(
    event: On<Activate>,
    picks: Query<&Pick>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Misc) {
        return;
    }
    let Ok(&Pick(topic)) = picks.get(event.entity) else {
        return;
    };
    state.help_topic = Some(topic);
    state.revision = state.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn help_topics_navigate_and_show_the_local_guide_text() -> TestResult {
        let mut app = super::super::tests::app(Page::Misc, None)?;
        for topic in Topic::ALL {
            let button = app.world_mut().spawn(Pick(topic)).observe(pick).id();
            app.world_mut().trigger(Activate { entity: button });
            assert_eq!(
                app.world().resource::<OptionsState>().help_topic,
                Some(topic)
            );
            let owner = app.world().non_send::<HostOwner>();
            assert!(!owner.settings.help_text(topic).is_empty());
        }
        Ok(())
    }
}
