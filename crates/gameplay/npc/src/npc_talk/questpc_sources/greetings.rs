//! Preserve Lubo's captured native handler over a quest-specific authored hint.
use super::super::RewardMessages;

pub(in crate::npc_talk) fn replace_authored_lubo(
    rows: &mut [RewardMessages],
    incoming: RewardMessages,
) -> Option<()> {
    // Crystal 201189f8 npc/lubo.lua:156; the source witness is retained in
    // questpc-citizen-reward-greetings.json. Other conflicts still fail closed.
    if incoming.npc != "oteryn:npc.lubo"
        || incoming.greet
            != "Welcome to my adventurer shop, |PLAYERNAME|! What do you need? Ask me for a {trade} to look at my wares."
        || incoming.farewell != "Good bye, |PLAYERNAME|."
        || incoming.walkaway != "Good bye."
    {
        return None;
    }
    let previous = rows.iter_mut().find(|row| row.npc == incoming.npc)?;
    if previous.greet != "Hello there. Are you looking for news of Hugo?"
        || previous.farewell != "Farewell."
        || previous.walkaway != "Another time, then."
    {
        return None;
    }
    *previous = incoming;
    Some(())
}
