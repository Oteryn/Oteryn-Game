//! S04c: Crystal 201189f public-channel access and Help moderation rules.
//! Group and account type are independent Control-issued facts, never wire input.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatRole {
    pub group_id: u8,
    pub account_type: u8,
    pub access: bool,
}

impl Default for ChatRole {
    fn default() -> Self {
        Self {
            group_id: 1,
            account_type: 1,
            access: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicRoom {
    World,
    English,
    Help,
    Advertising,
    AdvertisingRookgaard,
    Tutor,
    Gamemaster,
    GuildLeaders,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicChatFacts {
    pub role: ChatRole,
    pub level: u32,
    pub has_vocation: bool,
    pub premium: bool,
    pub guild_leader: bool,
}

impl PublicChatFacts {
    /// Crystal data/chatchannels/scripts/{advertising,advertising-rook,tutor,
    /// gamemaster,guild_leaders}.lua:1-3.
    pub fn can_join(self, room: PublicRoom) -> bool {
        match room {
            PublicRoom::World | PublicRoom::English | PublicRoom::Help => true,
            PublicRoom::Advertising => self.has_vocation || self.role.group_id >= 3,
            PublicRoom::AdvertisingRookgaard => !self.has_vocation || self.role.group_id >= 3,
            PublicRoom::Tutor => self.role.group_id >= 2,
            PublicRoom::Gamemaster => self.role.account_type >= 4,
            PublicRoom::GuildLeaders => self.guild_leader || self.role.access,
        }
    }

    /// Crystal scripts: world_chat:2-6, english_chat:2-5, help:8-11,
    /// advertising:12-23, advertising-rook:12-23. Joining and speaking differ.
    pub fn can_speak(self, room: PublicRoom) -> bool {
        self.can_join(room)
            && match room {
                PublicRoom::World => self.level != 1 || self.role.group_id >= 4,
                PublicRoom::English => self.level >= 20 || self.premium,
                PublicRoom::Help => self.level != 1 || self.role.group_id != 1,
                PublicRoom::Advertising => {
                    self.role.group_id >= 4 || self.level >= 20 || self.premium
                }
                PublicRoom::AdvertisingRookgaard => self.role.group_id >= 4 || self.level != 1,
                PublicRoom::Tutor | PublicRoom::Gamemaster | PublicRoom::GuildLeaders => true,
            }
    }
}

/// Crystal help.lua:3-5 and :28 intentionally have different durations.
pub const HELP_CONDITION_MS: u64 = 3_600_000;
pub const HELP_EXHAUSTION_MS: u64 = 180_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    Mute,
    Unmute,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HelpMute {
    pub condition_until_ms: u64,
    pub speech_until_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpRefusal {
    NotAuthorized,
    AlreadyMuted,
    NotMuted,
}

impl HelpMute {
    pub fn remaining_seconds(self, now_ms: u64) -> u32 {
        u32::try_from(self.speech_until_ms.saturating_sub(now_ms).div_ceil(1000))
            .unwrap_or(u32::MAX)
    }

    /// help.lua:20-60 compares the moderator's group to the target's account type.
    /// Chat::talkToChannel (chat.cpp:588-610) has no additional channel mute test:
    /// speech resumes after 180 s, while the 1 h condition blocks a second !mute.
    pub fn apply(
        self,
        actor: ChatRole,
        target: ChatRole,
        action: HelpAction,
        now_ms: u64,
    ) -> Result<Self, HelpRefusal> {
        if actor.group_id < 2 || actor.group_id <= target.account_type {
            return Err(HelpRefusal::NotAuthorized);
        }
        match action {
            HelpAction::Mute if self.condition_until_ms > now_ms => Err(HelpRefusal::AlreadyMuted),
            HelpAction::Mute => Ok(Self {
                condition_until_ms: now_ms.saturating_add(HELP_CONDITION_MS),
                speech_until_ms: now_ms.saturating_add(HELP_EXHAUSTION_MS),
            }),
            HelpAction::Unmute if self.speech_until_ms <= now_ms => Err(HelpRefusal::NotMuted),
            HelpAction::Unmute => Ok(Self::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn facts() -> PublicChatFacts {
        PublicChatFacts {
            role: ChatRole::default(),
            level: 1,
            has_vocation: false,
            premium: false,
            guild_leader: false,
        }
    }
    #[test]
    fn public_channels_donor_access_uses_distinct_group_account_and_guild_facts() {
        let ordinary = facts();
        assert!(ordinary.can_join(PublicRoom::AdvertisingRookgaard));
        assert!(!ordinary.can_join(PublicRoom::Advertising));
        for room in [
            PublicRoom::Tutor,
            PublicRoom::Gamemaster,
            PublicRoom::GuildLeaders,
        ] {
            assert!(!ordinary.can_join(room));
        }
        let tutor = PublicChatFacts {
            role: ChatRole {
                group_id: 2,
                ..ordinary.role
            },
            ..ordinary
        };
        assert!(tutor.can_join(PublicRoom::Tutor));
        assert!(!tutor.can_join(PublicRoom::Gamemaster));
        let senior = PublicChatFacts {
            role: ChatRole {
                group_id: 3,
                ..ordinary.role
            },
            ..ordinary
        };
        assert!(senior.can_join(PublicRoom::Advertising));
        assert!(senior.can_join(PublicRoom::AdvertisingRookgaard));
        let gm_account = PublicChatFacts {
            role: ChatRole {
                account_type: 4,
                ..ordinary.role
            },
            ..ordinary
        };
        assert!(gm_account.can_join(PublicRoom::Gamemaster));
        assert!(!gm_account.can_join(PublicRoom::Tutor));
        assert!(
            PublicChatFacts {
                guild_leader: true,
                ..ordinary
            }
            .can_join(PublicRoom::GuildLeaders)
        );
        assert!(
            PublicChatFacts {
                role: ChatRole {
                    access: true,
                    ..ordinary.role
                },
                ..ordinary
            }
            .can_join(PublicRoom::GuildLeaders)
        );
    }
    #[test]
    fn public_channels_speaking_level_premium_and_staff_exceptions_match_scripts() {
        let ordinary = facts();
        for room in [
            PublicRoom::World,
            PublicRoom::Help,
            PublicRoom::English,
            PublicRoom::AdvertisingRookgaard,
        ] {
            assert!(!ordinary.can_speak(room));
        }
        let level_two = PublicChatFacts {
            level: 2,
            ..ordinary
        };
        for room in [
            PublicRoom::World,
            PublicRoom::Help,
            PublicRoom::AdvertisingRookgaard,
        ] {
            assert!(level_two.can_speak(room));
        }
        assert!(!level_two.can_speak(PublicRoom::English));
        assert!(
            PublicChatFacts {
                premium: true,
                ..ordinary
            }
            .can_speak(PublicRoom::English)
        );
        assert!(
            PublicChatFacts {
                level: 20,
                ..ordinary
            }
            .can_speak(PublicRoom::English)
        );
        let gm = PublicChatFacts {
            role: ChatRole {
                group_id: 4,
                account_type: 4,
                access: true,
            },
            ..ordinary
        };
        for room in [
            PublicRoom::World,
            PublicRoom::Help,
            PublicRoom::Advertising,
            PublicRoom::AdvertisingRookgaard,
        ] {
            assert!(gm.can_speak(room));
        }
        assert!(!gm.can_speak(PublicRoom::English));
    }
    #[test]
    fn public_help_mute_preserves_both_donor_deadlines_and_strict_rank() {
        let tutor = ChatRole {
            group_id: 2,
            ..ChatRole::default()
        };
        let ordinary = ChatRole::default();
        let muted = HelpMute::default().apply(tutor, ordinary, HelpAction::Mute, 10_000);
        assert_eq!(
            muted,
            Ok(HelpMute {
                condition_until_ms: 3_610_000,
                speech_until_ms: 190_000
            })
        );
        let mute = muted.unwrap_or_default();
        assert_eq!(mute.remaining_seconds(10_000), 180);
        assert_eq!(mute.remaining_seconds(189_999), 1);
        assert_eq!(mute.remaining_seconds(190_000), 0);
        assert_eq!(
            mute.apply(tutor, ordinary, HelpAction::Mute, 190_000),
            Err(HelpRefusal::AlreadyMuted)
        );
        assert_eq!(
            mute.apply(tutor, ordinary, HelpAction::Unmute, 190_000),
            Err(HelpRefusal::NotMuted)
        );
        assert_eq!(
            mute.apply(tutor, ordinary, HelpAction::Unmute, 189_999),
            Ok(HelpMute::default())
        );
        assert!(
            mute.apply(tutor, ordinary, HelpAction::Mute, 3_610_000)
                .is_ok()
        );
        let peer = ChatRole {
            account_type: 2,
            ..ordinary
        };
        assert_eq!(
            HelpMute::default().apply(tutor, peer, HelpAction::Mute, 0),
            Err(HelpRefusal::NotAuthorized)
        );
        assert_eq!(
            HelpMute::default().apply(ordinary, ordinary, HelpAction::Mute, 0),
            Err(HelpRefusal::NotAuthorized)
        );
    }
}
