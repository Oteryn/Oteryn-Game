//! Read-only death protection receipt for the native post-death/login HUD.
mod protocol;
mod state;
mod view;
pub use protocol::decode;
pub use state::{DeathProtection, DeathProtectionReply};
pub use view::notice;
#[cfg(test)]
#[path = "tests/e2e_c20c.rs"]
mod e2e_c20c;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn death_protection_notice_uses_exact_receipt_consumption() {
        let mut value = DeathProtection {
            occurrence: [7; 16],
            consumed: vec!["oteryn:blessing.twist_of_fate".into()],
        };
        assert!(notice(&value).contains("Twist Of Fate"));
        value.consumed.clear();
        assert!(notice(&value).contains("No blessings or Twist of Fate"));
    }
}
