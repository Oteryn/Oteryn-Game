use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
/// Ctrl+M opens the Market panel, ignoring repeats and releases.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 16 && modifiers.contains(Modifier::Control)
            && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift)
            && !modifiers.contains(Modifier::Super))
    })
}

/// Parse the native form atomically so invalid input never submits stale details.
pub fn edit_details(
    panel: &mut super::MarketPanel,
    item: &str,
    amount: &str,
    price: &str,
    sell: bool,
    anonymous: bool,
) -> bool {
    let parsed = (
        item.trim().parse::<u32>(),
        amount.trim().parse::<u32>(),
        price.trim().parse::<u64>(),
    );
    if let (Ok(definition), Ok(amount), Ok(price)) = parsed
        && definition > 0
        && (1..=64_000).contains(&amount)
        && price > 0
        && u128::from(amount) * u128::from(price) <= 999_999_999_999
    {
        if panel.definition != definition {
            panel.page = 0;
        }
        panel.definition = definition;
        panel.amount = amount;
        panel.price = price;
        panel.sell = sell;
        panel.anonymous = anonymous;
        true
    } else {
        panel.feedback = Some(tr!(
            "Enter a positive item ID, amount (1–64000) and valid unit price"
        ));
        false
    }
}

pub fn coin_open_key(events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
    use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
    events.iter().any(|e|matches!(e,NormalizedInputEvent::Key{code,state:ButtonState::Pressed,modifiers,repeat:false} if code.get()==65 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}
