# Rafzan Guardcatcher offer

Reference checked: https://www.tibiawiki.com.br/wiki/Task:Rafzan#Guardian_Angel_-_Kind_Of
The captured task text specifies `Hi - Guards - Yes`, Rafzan's question, and the bow being supplied only after the player accepts. The two registered callbacks retain that order and the quoted English reply text. The existing native Guardcatcher identity is Item 17669; its physical source facts are documented in `guardcatcher-custody.md`.

The persisted `guard_offer` track is internal first-cycle progress: 0 before the question, 1 after the question, 2 after delivery. Topic 17 is an internal dialogue routing choice, not an asserted donor storage or topic ID. The existing read-only NPC offer path repeats the question at stage 1 after an interrupted conversation; it updates only the conversation topic and neither writes progress nor issues a tool. Repeating an old command replays its existing receipt. A new confirmation after delivery cannot mint another bow.

The registered-callback PostgreSQL case runs the real fenced reward writer with these production definitions, confirms the stage order, proves the MINT shares the confirmation transaction, and checks historical replay and fresh-command duplication refusal. It does not qualify NPC visibility or native conversation admission. Capture outcomes, the three distinct targets and their completion/failure branches require separate qualification; this offer does not invent the unspecified failure probability.
