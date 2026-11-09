use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_runtime::cui3::store::StoreCataloguePage;
use oteryn_client_ui::hireling_management::wire;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::play) struct Review {
    pub entry: wire::Entry,
    pub catalogue: StoreCataloguePage,
    pub index: usize,
    pub action: usize,
    pub name: String,
}
impl Review {
    pub fn text(&self) -> String {
        let offer = &self.catalogue.offers[self.index];
        format!(
            "{} for {}: {} {}?\nNew name: {}\nCurrent sex: {}\nService unlocks apply to all your hirelings.",
            offer.title, self.entry.name, offer.price, offer.currency, self.name, self.entry.sex
        )
    }
}
pub(in crate::play) fn review(
    m: &Mailbox,
    action: usize,
    name: &str,
) -> Result<Review, &'static str> {
    if !m.hireling_management.selected || !m.store.selected {
        return Err("Hireling configuration unavailable in this session");
    }
    if m.hireling_management.pending.is_some()
        || m.store.pending
        || m.store.confirmed_purchase.is_some()
    {
        return Err("Resolve the pending request before configuring a hireling");
    }
    if m.store.purchase.is_some() {
        return Err("Refresh owned hirelings after the configuration result");
    }
    let entry = m
        .hireling_management
        .entries
        .iter()
        .find(|e| Some(e.hireling) == m.hireling_management.identity)
        .ok_or("Select an owned hireling")?;
    let key = match action {
        131 => {
            if name == entry.name
                || !(3..=18).contains(&name.len())
                || !name.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
            {
                return Err("Enter a different name of 3–18 letters/spaces");
            }
            "store:hireling:25438".to_owned()
        }
        132 => "store:hireling:25437".into(),
        133..=136 => {
            if entry.skills & (1 << (action - 133)) != 0 {
                return Err("This service is already unlocked");
            }
            format!("store:hireling:{}", 1001 + action - 133)
        }
        _ => return Err("Invalid configuration action"),
    };
    if m.store.catalogue_category != "houses.hirelings" {
        return Err("Load hireling prices first");
    }
    let catalogue = m
        .store
        .catalogue
        .as_ref()
        .ok_or("Load hireling prices first")?;
    let index = catalogue
        .offers
        .iter()
        .position(|o| o.offer_key == key)
        .ok_or("This configuration offer is unavailable")?;
    Ok(Review {
        entry: entry.clone(),
        catalogue: catalogue.clone(),
        index,
        action,
        name: name.into(),
    })
}
pub(in crate::play) fn submit(
    m: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    sealed: &Review,
    confirmed: bool,
) -> bool {
    if !confirmed {
        return false;
    }
    if review(m, sealed.action, &sealed.name).as_ref() != Ok(sealed) {
        m.hireling_management.feedback = Some("Configuration or price changed; review again");
        return false;
    }
    let id = sealed
        .entry
        .hireling
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    crate::play::feature_0154_store::view::native::customization(
        &mut m.store,
        &sealed.name,
        "0",
        &id,
    );
    let Some(command) =
        crate::play::feature_0154_store::view::native::purchase(&mut m.store, sealed.index, true)
    else {
        return false;
    };
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        m.store.pending = false;
        m.hireling_management.feedback = Some("The session has ended");
        return false;
    }
    m.hireling_management.feedback =
        Some("Configuration submitted; refresh after the purchase result");
    true
}
#[cfg(windows)]
pub(in crate::play) fn prices(m: &mut Mailbox, commands: &UnboundedSender<PlayCommand>) {
    if m.store.pending || m.store.confirmed_purchase.is_some() {
        return;
    }
    m.store.category = "houses.hirelings".into();
    m.store.catalogue = None;
    if let Some(command) = oteryn_client_ui::store::catalogue(&mut m.store)
        && commands.send(PlayCommand::ServerMechanic(command)).is_err()
    {
        m.store.pending = false;
        m.hireling_management.feedback = Some("The session has ended");
    }
}
