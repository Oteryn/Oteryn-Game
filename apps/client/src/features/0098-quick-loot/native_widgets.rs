use super::*;

pub(super) unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}

pub(super) fn specs(
    panel: u32,
) -> (
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
) {
    match panel {
        98 => (
            &[
                tr!("Loot nearby"),
                tr!("Add item"),
                tr!("Remove item"),
                tr!("Clear list"),
                tr!("Select mode"),
                tr!("Set destination"),
                tr!("Lists / Managed containers"),
                tr!("Reset selected to main"),
            ],
            &[
                (tr!("Item type"), ""),
                (tr!("List: 1 skip, 2 accept"), "1"),
                (
                    tr!("Category: 0 all, 1 gold, 2 products, 3 gear, 4 consumables, 5 other"),
                    "0",
                ),
                (tr!("Fallback: 0 off, 1 on"), "1"),
                (tr!("Destination: 1 container, 2 gold pouch, 3 main"), "3"),
                (tr!("Container / pouch ID (if not main)"), ""),
            ],
        ),
        132 => (
            &[
                tr!("Inspect selected inventory item"),
                tr!("Permission requests"),
                tr!("Grant selected"),
                tr!("Revoke selected"),
                tr!("Allow everyone"),
                tr!("Disable allow everyone"),
                tr!("Inspect catalogue item"),
                tr!("Refresh inspection"),
                tr!("Inspect trade item"),
            ],
            &[
                (tr!("Catalogue item type"), ""),
                (tr!("Trade item index"), "0"),
                (tr!("Counter offer: 0 no, 1 yes"), "0"),
            ],
        ),
        232 => (
            &[
                tr!("Browse teams"),
                tr!("Your applicants"),
                tr!("Next page"),
                tr!("Request to join"),
                tr!("Cancel request"),
                tr!("Invite selected"),
                tr!("Accept selected"),
                tr!("Deny selected"),
                tr!("Publish advertisement"),
                tr!("Remove advertisement"),
                tr!("Apply team filters"),
                tr!("Clear team filters"),
            ],
            &[
                (tr!("Minimum level"), "1"),
                (tr!("Maximum level"), "1000"),
                (tr!("Vocations mask"), "255"),
                (tr!("Team slots"), "4"),
                (tr!("Free slots"), "3"),
                (tr!("Party members: 0 no, 1 yes"), "0"),
                (tr!("Target: 1 boss, 2 hunt, 3 quest"), "1"),
                (tr!("Boss / hunt kind / quest"), "0"),
                (tr!("Hunt area"), "0"),
                (tr!("Start time (Unix seconds; 0 now)"), "0"),
                (tr!("Filter leader name"), ""),
                (tr!("Filter level (0 any)"), "0"),
                (tr!("Filter vocations mask (0 any)"), "0"),
                (tr!("Filter target: 0 any/1 boss/2 hunt/3 quest"), "0"),
                (tr!("Filter only open: 0 no/1 yes"), "0"),
            ],
        ),
        _ => (
            &[
                tr!("Refresh current Store tab"),
                tr!("Next page in current tab"),
                tr!("Catalogue"),
                tr!("Purchase history"),
                tr!("Apply purchase history filters"),
            ],
            &[
                (tr!("Offer text (this page)"), ""),
                (tr!("From Unix milliseconds"), ""),
                (tr!("Through Unix milliseconds"), ""),
            ],
        ),
    }
}
