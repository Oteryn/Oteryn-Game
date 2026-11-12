//! Original category silhouettes, generated from confirmed catalogue facts; no imported artwork.
use oteryn_client_runtime::charm_progression::catalogue::Card;
/// A small category illustration that the native text panel can render without image assets.
pub fn illustration(card: &Card) -> String {
    let category = card.category.to_ascii_lowercase();
    let rows: &[&str] = if category.contains("undead") {
        &[
            "  █████  ",
            " █ █ █ █ ",
            " ███████ ",
            "  █ █ █  ",
            "   ███   ",
        ]
    } else if category.contains("dragon") || category.contains("demon") {
        &[
            "█       █",
            " ██ █ ██ ",
            "█████████",
            "  █████  ",
            "  █   █  ",
        ]
    } else if category.contains("insect") || category.contains("arachnid") {
        &[
            " █  █  █ ",
            "  █████  ",
            "█████████",
            "  █████  ",
            " █  █  █ ",
        ]
    } else if category.contains("humanoid") || category.contains("human") {
        &[
            "   ███   ",
            "   ███   ",
            " ███████ ",
            "   ███   ",
            "  ██ ██  ",
        ]
    } else {
        &[
            " █     █ ",
            " ███████ ",
            "██ █ █ ██",
            " ███████ ",
            "  █████  ",
        ]
    };
    format!(
        "{} — category illustration\r\n{}\r\n",
        card.name,
        rows.join("\r\n")
    )
}
