//! User actions preserve the selected server quote through confirmation and retries.
#![allow(unsafe_code, reason = "Controls remain on their creating thread")]
use super::{
    controls::{text, wide},
    store,
};
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
pub(super) unsafe fn handle(
    link: &PlayLink,
    hwnd: HWND,
    fields: [HWND; 7],
    actions: Vec<(usize, usize)>,
) {
    let [category, search, sort, offers, name, sex, identity] = fields;
    unsafe {
        for (id, note) in actions {
            if id == 9 && note == 0 {
                if let Some(launcher) = lock(&link.mailbox).coin_shop.clone() {
                    launcher.start();
                }
                continue;
            }
            if id == 14 && note == 0 {
                link.store_gift_open();
                continue;
            }
            if id == 12 && note == 0 {
                link.discovery_open(285);
                continue;
            }
            if id == 13 && note == 0 {
                let mut m = lock(&link.mailbox);
                if let Some(c) = store::coin_history(&mut m.store, false) {
                    let _ = link.commands.send(PlayCommand::ServerMechanic(c));
                }
                continue;
            }
            if id == 4 && note == 0 {
                let snapshot = link.store_panel();
                if snapshot.pending {
                    continue;
                }
                let quote = snapshot.catalogue.clone();
                let body = snapshot
                    .catalogue
                    .as_ref()
                    .and_then(|p| p.offers.first())
                    .map(|o| {
                        format!(
                            "Buy {} for {} {}?\r\nCatalogue: {}",
                            o.title,
                            o.price,
                            o.currency,
                            quote.as_ref().map_or("", |q| q.catalog_revision.as_str())
                        )
                    });
                if let Some(body) = body
                    && MessageBoxW(
                        hwnd,
                        wide(&body).as_ptr(),
                        wide("Confirm purchase").as_ptr(),
                        MB_YESNO | MB_ICONQUESTION,
                    ) == IDYES
                {
                    if let Some(quote) = &quote {
                        link.store_confirm_purchase(
                            quote,
                            &text(name),
                            &text(sex),
                            &text(identity),
                        );
                    }
                }
                continue;
            }
            if id == 3 && note == LBN_SELCHANGE as usize {
                let index = SendMessageW(offers, LB_GETCURSEL, 0, 0);
                if let Ok(index) = usize::try_from(index) {
                    store::select_product(&mut lock(&link.mailbox).store, index);
                }
                continue;
            }
            if matches!(id, 15 | 16) && note == 0 {
                let mut m = lock(&link.mailbox);
                if id == 15 {
                    m.store.browser.gender ^= 1;
                } else {
                    m.store.browser.direction = (m.store.browser.direction + 1) % 4;
                }
                continue;
            }
            let browse = (id == 1 && note == LBN_SELCHANGE as usize)
                || (id == 11 && note == CBN_SELCHANGE as usize)
                || (matches!(id, 2 | 5 | 6) && note == 0);
            if browse {
                let snapshot = link.store_panel();
                if snapshot.pending {
                    continue;
                }
                let key = if id == 1 {
                    usize::try_from(SendMessageW(category, LB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|i| {
                            snapshot
                                .browser
                                .page
                                .as_ref()?
                                .categories
                                .get(i)
                                .map(|c| c.key.clone())
                        })
                        .unwrap_or(snapshot.browser.category)
                } else {
                    snapshot.browser.category
                };
                let sort = u8::try_from(SendMessageW(sort, CB_GETCURSEL, 0, 0)).unwrap_or(0);
                link.store_browse(
                    &key,
                    &text(search),
                    sort,
                    if id == 5 {
                        -1
                    } else if id == 6 {
                        1
                    } else {
                        0
                    },
                );
            }
        }
    }
}
