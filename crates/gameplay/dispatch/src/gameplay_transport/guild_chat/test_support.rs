//! Small test-only wrapper around the existing private relay fixture.
use super::*;
pub(in crate::gameplay_transport) fn line(
    guild_id: crate::domain::guild::GuildId,
    text: &str,
) -> RelayLine {
    super::tests::relayed(
        RelayDestination::Guild {
            guild_id,
            rank: crate::domain::guild::GuildRank::VICE,
        },
        "Sender",
        text,
    )
}
