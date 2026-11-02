//! CLIENT-E2E-1 uses protocol-oteryn at the native client's wire boundary. The app and UI
//! remain outside the server dependency graph; each case names its app request builder.
//! This proves wire compatibility, not rendering or the app mailbox reducer.
#[path = "client_e2e/browse_field.rs"]
mod browse_field;
#[path = "client_e2e/daily_boost.rs"]
mod daily_boost;
#[path = "client_e2e/depot_search.rs"]
mod depot_search;
#[path = "client_e2e/hireling.rs"]
mod hireling;
#[path = "client_e2e/hotkey_equip.rs"]
mod hotkey_equip;
#[path = "client_e2e/item_forge.rs"]
mod item_forge;
#[path = "client_e2e/store.rs"]
mod store;
#[path = "client_e2e/support.rs"]
mod support;

use super::TestResult;
