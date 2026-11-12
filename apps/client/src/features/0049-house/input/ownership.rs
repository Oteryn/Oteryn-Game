use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::house::{parse_ownership_date, sale::Confirmation};
use tokio::sync::mpsc::UnboundedSender;
fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    command: FeatureCommand,
) {
    if commands.send(PlayCommand::Feature(command)).is_err() {
        let state = &mut mailbox.properties.house.ownership;
        state.model.ended();
        state.model.feedback = "The session has ended".into();
        state.navigation = None;
    }
}
pub(in crate::play) fn page(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    direction: i8,
) {
    match mailbox.properties.house.ownership.page(direction) {
        Ok(command) => enqueue(mailbox, commands, command),
        Err(error) => mailbox.properties.house.ownership.model.feedback = error.into(),
    }
}
pub(in crate::play) fn edit_fields(
    mailbox: &mut Mailbox,
    fields: [String; 3],
) -> Result<(), &'static str> {
    let state = &mut mailbox.properties.house.ownership;
    if state.model.pending() {
        return Err("Waiting for the server");
    }
    if fields[0].len() > 29
        || fields[1].len() > 40
        || fields[2].len() > 15
        || fields.iter().any(|f| f.contains('\0'))
    {
        return Err("Check the house transfer fields");
    }
    state.model.cancel_review();
    state.fields = fields;
    Ok(())
}
pub(in crate::play) fn review(
    mailbox: &mut Mailbox,
    action: u8,
    fields: [String; 3],
) -> Result<Confirmation, String> {
    edit_fields(mailbox, fields)?;
    let state = &mut mailbox.properties.house.ownership;
    let date = if matches!(action, 1 | 3) {
        parse_ownership_date(&state.fields[1])?
    } else {
        0
    };
    let price = if action == 3 {
        state.fields[2]
            .trim()
            .parse::<u64>()
            .map_err(|_| "Enter a gold price, including 0 for a gift")?
    } else {
        0
    };
    state
        .model
        .terms(state.fields[0].trim().to_owned(), date, price);
    state.model.prepare(action).map_err(str::to_owned)
}
pub(in crate::play) fn confirm(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    review: Confirmation,
    accepted: bool,
) {
    match mailbox
        .properties
        .house
        .ownership
        .model
        .confirm(review, accepted)
    {
        Ok(Some(command)) => enqueue(mailbox, commands, command),
        Ok(None) => {}
        Err(error) => mailbox.properties.house.ownership.model.feedback = error.into(),
    }
}
impl PlayLink {
    pub fn house_ownership_panel(&self) -> super::super::state::ownership::Ownership {
        lock(&self.mailbox).properties.house.ownership.clone()
    }
    pub fn house_ownership_open(&self) {
        #[cfg(windows)]
        super::super::view::ownership_window::open(self);
        #[cfg(not(windows))]
        {
            let mut mailbox = lock(&self.mailbox);
            if mailbox.properties.house.ownership.model.available() {
                mailbox.properties.house.ownership.open = true;
                page(&mut mailbox, &self.commands, 0);
            }
        }
    }
    pub fn house_ownership_page(&self, direction: i8) {
        page(&mut lock(&self.mailbox), &self.commands, direction);
    }
    pub fn house_ownership_select(&self, index: usize) -> Result<(), &'static str> {
        lock(&self.mailbox).properties.house.ownership.select(index)
    }
    pub fn house_ownership_review(
        &self,
        action: u8,
        fields: [String; 3],
    ) -> Result<Confirmation, String> {
        review(&mut lock(&self.mailbox), action, fields)
    }
    pub fn house_ownership_confirm(&self, review: Confirmation, accepted: bool) {
        confirm(&mut lock(&self.mailbox), &self.commands, review, accepted);
    }
}
