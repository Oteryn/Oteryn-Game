//! Native player-filter selectors; HWNDs stay on the battle window thread.
use super::super::preferences::GuildFilter;
use super::*;
pub(super) struct MetadataControls {
    vocation: HWND,
    guild: HWND,
}
impl MetadataControls {
    pub(super) unsafe fn new(parent: HWND) -> Option<Self> {
        unsafe {
            let label = control(
                parent,
                "STATIC",
                "Player profession / relationship to your Guild",
                0,
                (10, 550, 700, 25),
                0,
            );
            let vocation = control(
                parent,
                "COMBOBOX",
                "",
                310,
                (10, 578, 300, 200),
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let guild = control(
                parent,
                "COMBOBOX",
                "",
                311,
                (330, 578, 390, 180),
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            let help = control(
                parent,
                "STATIC",
                "Specific player filters exclude unknown details. Creature filtering remains separate.",
                0,
                (10, 615, 710, 35),
                0,
            );
            if [label, vocation, guild, help]
                .iter()
                .any(|handle| handle.is_null())
            {
                return None;
            }
            for text in [
                "Any profession",
                "No vocation",
                "Sorcerer",
                "Druid",
                "Paladin",
                "Knight",
                "Monk",
            ] {
                SendMessageW(vocation, CB_ADDSTRING, 0, wide(text).as_ptr() as isize);
            }
            for text in ["Any Guild relationship", "My Guild", "Outside my Guild"] {
                SendMessageW(guild, CB_ADDSTRING, 0, wide(text).as_ptr() as isize);
            }
            SendMessageW(vocation, CB_SETCURSEL, 0, 0);
            SendMessageW(guild, CB_SETCURSEL, 0, 0);
            Some(Self { vocation, guild })
        }
    }
    pub(super) unsafe fn values(&self) -> (Option<u8>, GuildFilter) {
        unsafe {
            let selected = SendMessageW(self.vocation, CB_GETCURSEL, 0, 0);
            let vocation = if (1..=6).contains(&selected) {
                u8::try_from(selected - 1).ok()
            } else {
                None
            };
            let guild = match SendMessageW(self.guild, CB_GETCURSEL, 0, 0) {
                1 => GuildFilter::SameGuild,
                2 => GuildFilter::OutsideGuild,
                _ => GuildFilter::Any,
            };
            (vocation, guild)
        }
    }
    pub(super) unsafe fn set(&self, preferences: &BattleListPreferences) {
        unsafe {
            SendMessageW(
                self.vocation,
                CB_SETCURSEL,
                preferences
                    .vocation
                    .map_or(0, |vocation| usize::from(vocation) + 1),
                0,
            );
            SendMessageW(
                self.guild,
                CB_SETCURSEL,
                match preferences.guild {
                    GuildFilter::Any => 0,
                    GuildFilter::SameGuild => 1,
                    GuildFilter::OutsideGuild => 2,
                },
                0,
            );
        }
    }
}
