//! Capability 489 `HOUSE_PODIUM_FLAGS_V1`: command 175 and the 174/176 projections carry the three
//! podium switches (platform, outfit, mount). Without the capability the wire stays the V1 one.
use crate::house_podium::Panel;
use oteryn_client_runtime::cui3::house_podium::{DisplayFlags, Projection, SetRequest};

/// One of the three podium switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Platform,
    Outfit,
    Mount,
}

/// Flip one switch of the draft. Refused without the capability or while a request is pending.
pub fn toggle(panel: &mut Panel, part: Part) -> bool {
    if !panel.flagged || panel.pending {
        return false;
    }
    let flag = match part {
        Part::Platform => &mut panel.flags.show_platform,
        Part::Outfit => &mut panel.flags.show_outfit,
        Part::Mount => &mut panel.flags.show_mount,
    };
    *flag = !*flag;
    true
}

/// The command 175 payload: the V1 request, followed by the three flags when selected.
#[must_use]
pub fn set_payload(panel: &Panel, request: &SetRequest) -> Option<Vec<u8>> {
    if panel.flagged {
        request.encode_flagged(panel.flags)
    } else {
        request.encode()
    }
}

/// The 174/176 reply: the committed display and, when selected, its flags.
#[must_use]
pub fn decode_projection(
    panel: &Panel,
    payload: &[u8],
) -> Option<Option<(Projection, Option<DisplayFlags>)>> {
    if panel.flagged {
        return Some(Projection::decode_flagged(payload)?.map(|(p, f)| (p, Some(f))));
    }
    Some(Projection::decode(payload)?.map(|p| (p, None)))
}

/// The switch line of the panel text, shown only when the capability is selected.
#[must_use]
pub fn flags_line(panel: &Panel) -> Option<String> {
    if !panel.flagged {
        return None;
    }
    let state = |on: bool| if on { "shown" } else { "hidden" };
    Some(format!(
        "Platform {} | outfit {} | mount {}",
        state(panel.flags.show_platform),
        state(panel.flags.show_outfit),
        state(panel.flags.show_mount)
    ))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
