use crate::play::Pushed;
use oteryn_session::{AppliedDelta, guild_emblem::GuildEmblem};
pub(in crate::play) fn emblems(pushed: &mut Pushed, delta: AppliedDelta<Vec<GuildEmblem>>) {
    oteryn_client_ui::guild_emblem::apply(&mut pushed.guild_emblems, delta.value);
}
