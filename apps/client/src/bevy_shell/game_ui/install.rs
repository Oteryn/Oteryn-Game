//! Installation of the shared in-game presentations.
use super::{
    actions, analyser, bug_report, chat, cyclopedia, depot, feature_dialogs, forge, highscores,
    hotkeys, hud, imbuing, inventory, item_details, journal, market, minimap, modal, npc_shop,
    options, outfit, party, prey, proficiency, project, projection, quick_loot, reward_wall,
    rule_violation, satellite_map, stash, store, task_board, trade, tutorial, wheel,
};
use bevy::prelude::*;

pub(in crate::bevy_shell) fn install(app: &mut App) {
    actions::install(app);
    bug_report::install(app);
    chat::install(app);
    cyclopedia::install(app);
    analyser::install(app);
    depot::install(app);
    feature_dialogs::install(app);
    highscores::install(app);
    forge::install(app);
    hotkeys::install(app);
    hud::install(app);
    imbuing::install(app);
    inventory::install(app);
    item_details::install(app);
    journal::install(app);
    market::install(app);
    minimap::install(app);
    modal::install(app);
    npc_shop::install(app);
    options::install(app);
    outfit::install(app);
    party::install(app);
    prey::install(app);
    proficiency::install(app);
    quick_loot::install(app);
    task_board::install(app);
    stash::install(app);
    rule_violation::install(app);
    store::install(app);
    satellite_map::install(app);
    reward_wall::install(app);
    trade::install(app);
    wheel::install(app);
    tutorial::install(app);
    app.add_systems(
        Update,
        (
            projection::missing_item_appearances,
            projection::project_regions.after(project),
        ),
    );
}
