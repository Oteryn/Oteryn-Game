//! Project shell controls into the in-game toolbar.
use super::{Control, ShellToolbar, Status};
use crate::bevy_shell::{game_ui, owner::HostOwner};
use bevy::prelude::*;

/// Keep admission controls outside the in-game HUD once play is installed.
// One disjoint query changes the toolbar, status and admission button nodes together.
#[allow(clippy::type_complexity)]
pub(in crate::bevy_shell) fn project_toolbar(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Query<Entity, With<game_ui::ToolbarRegion>>,
    mut nodes: Query<
        (
            Entity,
            &mut Node,
            Has<ShellToolbar>,
            Has<Status>,
            Option<&Control>,
            Option<&ChildOf>,
        ),
        Or<(With<ShellToolbar>, With<Status>, With<Control>)>,
    >,
) {
    let playing = owner.play.is_some();
    let staff = game_ui::staff_tools::toolbar_visible(&owner);
    for (entity, mut node, toolbar, status, control, parent) in &mut nodes {
        if toolbar {
            if playing {
                if let Ok(region) = regions.single() {
                    if parent.is_none_or(|parent| parent.parent() != region) {
                        commands.entity(entity).insert(ChildOf(region));
                    }
                    node.position_type = PositionType::Relative;
                    node.width = percent(100.);
                    node.max_width = percent(100.);
                    node.overflow = Overflow::clip();
                    node.top = Val::Auto;
                    node.bottom = Val::Auto;
                    node.left = Val::Auto;
                    node.right = Val::Auto;
                }
            } else {
                if parent.is_some() {
                    commands.entity(entity).remove::<ChildOf>();
                }
                node.position_type = PositionType::Absolute;
                node.width = Val::Auto;
                node.max_width = Val::Auto;
                node.top = px(8.);
                node.bottom = Val::Auto;
                node.left = px(8.);
                node.right = px(8.);
            }
        } else if matches!(
            control,
            Some(
                Control::Appearance
                    | Control::StatusDetails
                    | Control::CastChat
                    | Control::Cast
                    | Control::StaffTools
                    | Control::PlayerReport
                    | Control::WorldEvent
                    | Control::ExivaSettings
                    | Control::TeamFinder
                    | Control::Training
                    | Control::HirelingServices
                    | Control::FurnitureRotation
                    | Control::HouseMoveOut
                    | Control::HouseTransfer
                    | Control::HouseBrowsing
                    | Control::GuildhallLeadership
                    | Control::GuildWars
                    | Control::CyclopediaTrackers
                    | Control::CyclopediaMap
                    | Control::BossDifficulty
                    | Control::BossRewardChest
                    | Control::AnimusMastery
                    | Control::BrowseField
                    | Control::ContainerSorting
                    | Control::Market
                    | Control::Party
                    | Control::Quests
                    | Control::WeaponProficiency
                    | Control::Bosstiary(_)
                    | Control::AchievementNotices
                    | Control::ExpertPvpModes
                    | Control::Guild(_)
                    | Control::RaidSchedule
                    | Control::Arena
                    | Control::CyclopediaCharacter
                    | Control::FurnitureWrapping
                    | Control::Spells
            )
        ) {
            node.display = if playing && (staff || !matches!(control, Some(Control::StaffTools))) {
                Display::Flex
            } else {
                Display::None
            };
        } else if status || matches!(control, Some(Control::Login | Control::CancelLogin)) {
            node.display = if playing {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}
