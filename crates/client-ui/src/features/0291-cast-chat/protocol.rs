use super::Panel;
use crate::cast_control::Panel as Control;
use oteryn_client_runtime::{
    CastCasterLine, cast::Message, server_mechanic::ServerMechanicCommand,
};
pub fn append(panel: &mut Panel, line: CastCasterLine) {
    if !panel.selected {
        return;
    }
    panel.lines.push_back(line);
    if panel.lines.len() > 128 {
        panel.lines.pop_front();
    }
}
pub fn command(
    panel: &Panel,
    control: &Control,
    text: &str,
    subject: &str,
    action: u8,
    key: [u8; 16],
) -> Result<ServerMechanicCommand, &'static str> {
    if !panel.selected || panel.pending || !control.active {
        return Err(tr!("No active cast chat"));
    }
    let (capability, command_type, message) = if action == 0 {
        (291, 504, Message::Reply(text.to_owned()))
    } else {
        if !(1..=4).contains(&action) || !subject.is_ascii() || subject.len() != 32 {
            return Err(tr!("Select a viewer handle from cast chat"));
        }
        let mut handle = [0; 16];
        for (i, byte) in handle.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&subject[i * 2..i * 2 + 2], 16)
                .map_err(|_| tr!("Invalid viewer handle"))?;
        }
        if handle == [0; 16] || !panel.subjects.contains(&handle) {
            return Err(tr!("Unknown viewer handle"));
        }
        (
            290,
            505,
            Message::Moderate {
                revision: control.revision,
                subject: handle,
                action,
                key,
            },
        )
    };
    let payload = message
        .encode()
        .map_err(|_| tr!("Invalid cast text or moderation request"))?;
    Ok(ServerMechanicCommand {
        capability,
        command_type,
        payload,
    })
}

pub fn roster(panel: &mut Panel, roster: oteryn_client_runtime::CastRoster) {
    if panel.selected && roster.revision >= panel.roster_revision {
        panel.roster_revision = roster.revision;
        panel.subjects = roster.subjects;
    }
}
