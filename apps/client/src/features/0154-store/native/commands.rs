//! User actions preserve the selected server quote through confirmation and retries.
#![allow(unsafe_code, reason = "Controls remain on their creating thread")]
#[cfg(windows)]
use super::{
    controls::{text, wide},
    store,
};
#[cfg(windows)]
use crate::{
    play::{PlayLink, mailbox::lock},
};
#[cfg(windows)]
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
#[cfg(windows)]
pub(super) unsafe fn handle(
    link: &PlayLink,
    hwnd: HWND,
    fields: [HWND; 7],
    actions: Vec<(usize, usize)>,
    displayed_quote: Option<&oteryn_client_runtime::cui3::store::StoreCataloguePage>,
) {
    let [category, search, sort, offers, name, sex, identity] = fields;
    unsafe {
        for (id, note) in actions {
            if id == 17 && note == 0 {
                let tokens = !link
                    .store_panel()
                    .catalogue
                    .as_ref()
                    .and_then(|p| p.offers.first())
                    .is_some_and(|o| o.currency == "COSMETIC_TOKEN");
                link.store_payment(tokens);
                continue;
            }
            if id == CLASSIC_STORE && note == 0 {
                super::super::open_classic_window(link);
                continue;
            }
            if id == 9 && note == 0 {
                if let Some(launcher) = lock(&link.mailbox).coin_shop.clone() {
                    launcher.start();
                }
                continue;
            }
            if (id == 24 || id == 25) && note == 0 {
                if let Some(launcher) = lock(&link.mailbox).coin_shop.clone() {
                    launcher.open_web(if id == 24 {
                        crate::web_links::WebLink::CoinHistory
                    } else {
                        crate::web_links::WebLink::CoinTransfer
                    });
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
            if history_control(link, id, note) {
                continue;
            }
            if id == 4 && note == 0 {
                let snapshot = link.store_panel();
                if snapshot.pending {
                    continue;
                }
                let Some(expected) = displayed_quote else {
                    continue;
                };
                if store::confirmation_quote(&snapshot).as_ref() != Some(expected) {
                    continue;
                }
                let quote = Some(expected.clone());
                let body = quote.as_ref().and_then(|p| p.offers.first()).map(|o| {
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
                        wide(tr!("Confirm purchase")).as_ptr(),
                        MB_YESNO | MB_ICONQUESTION,
                    ) == IDYES
                    && let Some(quote) = &quote
                {
                    link.store_confirm_purchase(quote, &text(name), &text(sex), &text(identity));
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
            if preview_control(&mut lock(&link.mailbox).store, id, note) {
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

pub(in crate::play) const CLASSIC_STORE: usize = 18;
pub(in crate::play) fn preview_control(
    panel: &mut oteryn_client_ui::store::StorePanel,
    id: usize,
    note: usize,
) -> bool {
    if note != 0 || id == CLASSIC_STORE {
        return false;
    }
    match id {
        15 => panel.browser.gender ^= 1,
        16 => panel.browser.direction = (panel.browser.direction + 1) % 4,
        _ => return false,
    }
    true
}

pub(in crate::play) fn history_control(
    link: &crate::play::PlayLink,
    id: usize,
    note: usize,
) -> bool {
    if !matches!(id, 13 | 27) || note != 0 {
        return false;
    }
    link.coin_history(id == 27)
}
