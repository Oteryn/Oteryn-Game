//! Capability 9 uses the shared admitted-session command exchange.
use crate::server_mechanic::ServerMechanicCommand;
pub use oteryn_session::highscores as wire;
pub fn command(
    query: wire::HighscoresQuery,
) -> Result<ServerMechanicCommand, wire::HighscoresWireError> {
    Ok(ServerMechanicCommand {
        capability: wire::CAPABILITY_HIGHSCORES_V1,
        command_type: wire::COMMAND_TYPE_HIGHSCORES_QUERY_V1,
        payload: wire::encode_highscores_query(query)?,
    })
}
