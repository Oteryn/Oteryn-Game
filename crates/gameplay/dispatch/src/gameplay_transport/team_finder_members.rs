//! S14b/S14c commands extend the existing leader listing and durable session owner.
#[path = "team_finder_members/actor.rs"]
mod actor;
#[path = "team_finder_members/commands.rs"]
mod commands;
#[path = "team_finder_members/handles.rs"]
mod handles;
#[path = "team_finder_members/projection.rs"]
mod projection;

pub(super) use commands::execute;
pub use handles::SessionTeamFinderView;

macro_rules! authority_methods {
    () => {
        fn team_finder_members_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command: u64,
            _command_type: u32,
            _payload: &[u8],
            _view: &mut $crate::gameplay_transport::team_finder_members::SessionTeamFinderView,
        ) -> impl std::future::Future<Output = Option<(bool, Vec<u8>)>> {
            async { Some((false, Vec::new())) }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        async fn team_finder_members_command(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command: u64,
            command_type: u32,
            payload: &[u8],
            view: &mut $crate::gameplay_transport::team_finder_members::SessionTeamFinderView,
        ) -> Option<(bool, Vec<u8>)> {
            team_finder_members::execute(self, admitted, command, command_type, payload, view).await
        }
    };
}
pub(super) use composed_methods;
