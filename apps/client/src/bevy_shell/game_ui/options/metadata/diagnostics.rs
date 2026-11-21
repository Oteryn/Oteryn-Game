use super::*;
/// The log level and the overlay; the log-folder and copy buttons sit below these rows.
pub(super) const ROWS: [Row; 2] = [
    Row {
        id: "diagnostics.log_level",
        label: "Log Level",
        editor: Editor::Choice {
            read: |p| usize::from(p.diagnostics.log_level.min(2)),
            // Row::set checks the index against the three values before this conversion.
            write: |p, v| p.diagnostics.log_level = v as u8,
            values: &["Warning", "Info", "Debug"],
        },
    },
    check!(
        "diagnostics.overlay",
        "Show diagnostics overlay",
        diagnostics.overlay
    ),
];
