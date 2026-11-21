//! Incoming grants use server consent handles and outgoing grants use contact entries.
use super::*;

#[derive(Component)]
pub(super) struct Section;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Accept(u64),
    Reject(u64),
    Revoke(u64),
    Presence(bool),
}

impl Action {
    pub(super) fn intent(self, snapshot: &Snapshot) -> Option<Intent> {
        let has_consent = |handle, accepted| {
            handle != 0
                && snapshot
                    .consents
                    .iter()
                    .any(|row| row.handle == handle && row.accepted == accepted)
        };
        match self {
            Self::Accept(handle) if has_consent(handle, false) => Some(Intent::Accept(handle)),
            Self::Reject(handle) if has_consent(handle, false) => Some(Intent::Reject(handle)),
            Self::Revoke(handle) if has_consent(handle, true) => Some(Intent::Revoke(handle)),
            Self::Presence(shown) => Some(Intent::Status(if shown { 1 } else { 2 })),
            _ => None,
        }
    }
}

pub(super) fn setup(mut commands: Commands, list: Query<Entity, With<List>>) {
    let Ok(list) = list.single() else { return };
    let section = commands
        .spawn((
            Section,
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(list).add_child(section);
}

#[derive(PartialEq, Eq)]
pub(super) struct Projection {
    generation: u64,
    revision: u64,
    disabled: bool,
    shown: Option<bool>,
    consents: Vec<model::Consent>,
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    sections: Query<Entity, With<Section>>,
    mut old: Local<Option<Projection>>,
) {
    let Ok(section) = sections.single() else {
        return;
    };
    let next = owner.play.as_ref().and_then(|play| {
        let panel = play.link.vip_panel();
        if !panel.selected {
            return None;
        }
        let snapshot = Snapshot::read(&panel).ok()?;
        Some(Projection {
            generation: owner.generation(),
            revision: panel.revision,
            disabled: panel.pending || !available(&owner),
            shown: snapshot.shown,
            consents: snapshot.consents,
        })
    });
    if *old == next {
        return;
    }
    *old = next;
    commands.entity(section).despawn_children();
    let Some(model) = old.as_ref() else { return };
    let caption = |commands: &mut Commands, value: &str| {
        let label = commands
            .spawn(source::text(value, source::font(skins.font.clone(), false)))
            .id();
        commands.entity(section).add_child(label);
    };
    let control = |commands: &mut Commands, action, label, id| {
        let entity = button(
            commands,
            &skins,
            Control {
                generation: model.generation,
                revision: model.revision,
                command: super::Action::Consent(action),
            },
            label,
            130.,
            id,
        );
        if model.disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
        commands.entity(section).add_child(entity);
    };
    if let Some(shown) = model.shown {
        caption(
            &mut commands,
            if shown {
                "VIP presence is visible"
            } else {
                "VIP presence is hidden"
            },
        );
        control(
            &mut commands,
            Action::Presence(true),
            "Appear online",
            "vip.presence.online",
        );
        control(
            &mut commands,
            Action::Presence(false),
            "Appear offline",
            "vip.presence.offline",
        );
    }
    for row in model
        .consents
        .iter()
        .filter(|row| row.handle != 0 && !row.name.is_empty())
    {
        let status = if row.accepted {
            "Access granted"
        } else {
            "Incoming request"
        };
        caption(&mut commands, &format!("{status} — {}", row.name));
        if row.accepted {
            control(
                &mut commands,
                Action::Revoke(row.handle),
                "Revoke",
                "vip.consent.revoke",
            );
        } else {
            control(
                &mut commands,
                Action::Accept(row.handle),
                "Accept",
                "vip.consent.accept",
            );
            control(
                &mut commands,
                Action::Reject(row.handle),
                "Reject",
                "vip.consent.reject",
            );
        }
    }
}
