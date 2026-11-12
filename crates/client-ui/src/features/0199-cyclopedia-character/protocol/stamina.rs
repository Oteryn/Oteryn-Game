//! C15: display the server's stamina minutes without inferring XP multipliers.
//! Crystal 201189f8 src/server/network/protocol/protocolgame.cpp:4401-4405.
pub(super) fn display(minutes: u16) -> String {
    lfmt!(
        "Stamina: {}:{:02} (hours:minutes)",
        minutes / 60,
        minutes % 60
    )
}
