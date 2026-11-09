use oteryn_client_runtime::cui3::store::{
    StoreCataloguePage, StoreClaimReceipt, StoreHistoryPage, StorePurchaseResult,
};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StorePage {
    #[default]
    Catalogue,
    History,
    CoinHistory,
    Inbox,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorePanel {
    pub coin_history: super::CoinHistory,
    pub gift_open: bool,
    pub gift_fields: [String; 2],
    pub gift_reviewed: Option<GiftDraft>,
    pub search: String,
    pub sort: StoreSort,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_action: u32,
    pub page: StorePage,
    pub category: String,
    pub selected_offer: usize,
    pub catalogue_category: String,
    pub hireling_name: String,
    pub hireling_sex: u8,
    pub hireling_id: Option<[u8; 16]>,
    /// Frozen confirmed wire request; uncertain outcomes retry these exact bytes.
    pub confirmed_purchase: Option<Vec<u8>>,
    pub purchase_request_id: [u8; 16],
    pub purchase_confirmed: bool,
    pub fulfilment_unit_id: String,
    pub catalogue: Option<StoreCataloguePage>,
    pub history: Option<StoreHistoryPage>,
    pub purchase: Option<StorePurchaseResult>,
    pub claim: Option<StoreClaimReceipt>,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StoreSort {
    #[default]
    Catalogue,
    Title,
    Price,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiftDraft {
    pub recipient: String,
    pub amount: u64,
}

fn gift_draft(
    panel: &StorePanel,
    recipient: &str,
    amount: &str,
) -> Result<GiftDraft, &'static str> {
    if !panel.selected {
        return Err(tr!("Store unavailable in this session"));
    }
    if panel.pending {
        return Err(tr!("Waiting for the server"));
    }
    let recipient = recipient.trim();
    if recipient.is_empty() || recipient.len() > 64 || recipient.chars().any(char::is_control) {
        return Err(tr!("Enter a recipient name of at most 64 bytes"));
    }
    let amount = amount
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|amount| *amount > 0)
        .ok_or(tr!("Enter a positive coin amount"))?;
    let balance = panel
        .catalogue
        .as_ref()
        .ok_or(tr!("Load the Store balance before reviewing a gift"))?
        .transferable_coins;
    if amount > balance {
        return Err(tr!("The amount exceeds your transferable coin balance"));
    }
    Ok(GiftDraft {
        recipient: recipient.into(),
        amount,
    })
}

pub fn review_gift(panel: &mut StorePanel, recipient: &str, amount: &str) -> bool {
    panel.gift_reviewed = None;
    match gift_draft(panel, recipient, amount) {
        Ok(draft) => {
            panel.feedback = Some(lfmt!(
                "Review gift to {}: {} transferable coins. Transfers are unavailable in this session.",
                draft.recipient,
                draft.amount
            ));
            panel.gift_reviewed = Some(draft);
            true
        }
        Err(message) => {
            panel.feedback = Some(message.into());
            false
        }
    }
}

/// No transfer intent is emitted until an accepted Platform gift contract exists.
pub fn confirm_gift(panel: &mut StorePanel, recipient: &str, amount: &str) -> bool {
    let draft = match gift_draft(panel, recipient, amount) {
        Ok(draft) => draft,
        Err(message) => {
            panel.gift_reviewed = None;
            panel.feedback = Some(message.into());
            return false;
        }
    };
    if panel.gift_reviewed.as_ref() != Some(&draft) {
        panel.gift_reviewed = None;
        panel.feedback =
            Some(tr!("Review the exact recipient and amount before confirming").into());
        return false;
    }
    panel.gift_reviewed = None;
    panel.feedback =
        Some(tr!("Coin transfers are unavailable in this session. No coins were moved.").into());
    false
}
