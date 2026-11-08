pub(super) fn specs(
    panel: u32,
) -> (
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
) {
    match panel {
        98 => (
            &[
                "Loot nearby",
                "Add item",
                "Remove item",
                "Clear list",
                "Select mode",
                "Set destination",
                "Lists / Managed containers",
                "Reset selected to main",
            ],
            &[
                ("Item type", ""),
                ("List: 1 skip, 2 accept", "1"),
                (
                    "Category: 0 all, 1 gold, 2 products, 3 gear, 4 consumables, 5 other",
                    "0",
                ),
                ("Fallback: 0 off, 1 on", "1"),
                ("Destination: 1 container, 2 gold pouch, 3 main", "3"),
                ("Container / pouch ID (if not main)", ""),
            ],
        ),
        132 => (
            &[
                "Inspect selected inventory item",
                "Permission requests",
                "Grant selected",
                "Revoke selected",
                "Allow everyone",
                "Disable allow everyone",
                "Inspect catalogue item",
                "Refresh inspection",
                "Inspect trade item",
            ],
            &[
                ("Catalogue item type", ""),
                ("Trade item index", "0"),
                ("Counter offer: 0 no, 1 yes", "0"),
            ],
        ),
        232 => (
            &[
                "Join team — Browse",
                "Assemble team — Applicants",
                "Next page",
                "Request to join",
                "Cancel request",
                "Invite selected",
                "Accept selected",
                "Deny selected",
                "Publish advertisement",
                "Remove advertisement",
                "Apply team filters",
                "Clear team filters",
            ],
            &[
                ("Minimum level", "1"),
                ("Maximum level", "1000"),
                ("Vocations mask", "255"),
                ("Team slots", "4"),
                ("Free slots", "3"),
                ("Party members: 0 no, 1 yes", "0"),
                ("Target: 1 boss, 2 hunt, 3 quest", "1"),
                ("Boss / hunt kind / quest", "0"),
                ("Hunt area", "0"),
                ("Start time (Unix seconds; 0 now)", "0"),
                ("Filter leader name", ""),
                ("Filter level (0 any)", "0"),
                ("Filter vocations mask (0 any)", "0"),
                ("Filter target: 0 any/1 boss/2 hunt/3 quest", "0"),
                ("Filter only open: 0 no/1 yes", "0"),
            ],
        ),
        _ => (
            &[
                "Refresh current Store tab",
                "Next page in current tab",
                "Catalogue",
                "Purchase history",
                "Apply purchase history filters",
            ],
            &[
                ("Offer text (this page)", ""),
                ("From Unix milliseconds", ""),
                ("Through Unix milliseconds", ""),
            ],
        ),
    }
}
