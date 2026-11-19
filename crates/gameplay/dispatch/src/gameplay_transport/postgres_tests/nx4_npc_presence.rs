//! NX-4: committed source voices through the production Channel cadence and bounded CHAT queue.
use super::nx4_npc_presence_fixtures::{source_voice_case};

use super::TestResult;

#[test]
fn nx4_source_npc_voice_reaches_chat_once_per_channel_think_on_postgres() -> TestResult {
    source_voice_case("oteryn:npc.captain_bluebear", "Captain Bluebear", "Passages to Carlin, Ab'Dendriel")
}
