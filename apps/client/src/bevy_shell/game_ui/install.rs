//! Installation of the shared in-game presentations.
use super::{
    actions, analyser, bestiary, bosstiary, bug_report, chat, cyclopedia, cyclopedia_items, cyclopedia_map, depot, feature_dialogs, forge, highscores, hotkeys, hud, imbuing, inventory, item_details, journal, layout, market, minimap, modal, npc_shop, options, outfit, party, prey, proficiency, project, projection, quick_loot, reward_wall, rule_violation, satellite_map, stash, store, store_inbox, task_board, trade, tutorial, unjustified_points, wheel,
};
use bevy::prelude::*;

pub(in crate::bevy_shell) fn install(app: &mut App) {
    bosstiary::install(app);
    actions::install(app);
    bestiary::install(app);
    bug_report::install(app);
    chat::install(app);
    cyclopedia::install(app);
    cyclopedia_items::install(app);
    cyclopedia_map::install(app);
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
    layout::install(app);
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
    store_inbox::install(app);
    satellite_map::install(app);
    reward_wall::install(app);
    trade::install(app);
    unjustified_points::install(app);
    wheel::install(app);
    tutorial::install(app);
    app.add_systems(
        Update,
        (
            projection::prepare_item_appearances.before(super::super::scene::project),
            projection::item_appearances.after(super::super::scene::project),
            projection::project_regions.after(project),
        ),
    );
}
