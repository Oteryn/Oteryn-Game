use super::ChatPanel;
use oteryn_client_runtime::chat_private_channel::PrivateChannelIntent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Create,
    Invite,
    Exclude,
    Join,
    Leave,
    Send,
    List,
}

pub fn request(panel: &mut ChatPanel, intent: Intent) -> Option<PrivateChannelIntent> {
    if !panel.selected || panel.pending {
        panel.feedback = Some(tr!("Private channels unavailable or waiting for the server").into());
        return None;
    }
    let command = match intent {
        Intent::Create => PrivateChannelIntent::Create,
        Intent::List => PrivateChannelIntent::List,
        Intent::Invite | Intent::Exclude => {
            let name = panel.name.trim();
            if name.is_empty() || name.len() > 120 || name.contains('\0') {
                panel.feedback = Some(tr!("Enter a player name of at most 120 bytes").into());
                return None;
            }
            if intent == Intent::Invite {
                PrivateChannelIntent::Invite { name: name.into() }
            } else {
                PrivateChannelIntent::Exclude { name: name.into() }
            }
        }
        Intent::Join | Intent::Leave | Intent::Send => {
            let Some(channel_id) = panel.channel else {
                panel.feedback = Some(tr!("Select your channel or an invitation").into());
                return None;
            };
            match intent {
                Intent::Join => PrivateChannelIntent::Join { channel_id },
                Intent::Leave => PrivateChannelIntent::Leave { channel_id },
                _ => {
                    if panel.text.trim().is_empty()
                        || panel.text.len() > 1_020
                        || panel.text.contains('\0')
                    {
                        panel.feedback = Some(tr!("Enter a message of at most 1,020 bytes").into());
                        return None;
                    }
                    PrivateChannelIntent::Say {
                        channel_id,
                        text: panel.text.clone(),
                    }
                }
            }
        }
    };
    panel.pending = true;
    panel.pending_intent = Some(intent);
    panel.feedback = None;
    Some(command)
}
