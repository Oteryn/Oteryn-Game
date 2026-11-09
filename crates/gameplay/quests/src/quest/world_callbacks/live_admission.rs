//! One transaction-bound Premium owner admits both historical and structured guards.
use super::Callback;

impl Callback {
    pub fn requires_live_premium(&self) -> bool {
        self.requires_premium || self.admission.as_ref().is_some_and(|guard| guard.premium)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Admission, load};

    #[test]
    fn m03_structured_premium_uses_the_same_live_gate_as_legacy_callbacks() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("/behemoth"))
            .ok_or("template")?;
        callback.requires_premium = false;
        callback.admission = Some(Admission {
            regular_blessings: false,
            promotion: false,
            premium: true,
        });
        assert!(
            callback.requires_live_premium(),
            "structured Premium must reach the live gate"
        );
        callback.admission = None;
        callback.requires_premium = true;
        assert!(callback.requires_live_premium());
        callback.requires_premium = false;
        assert!(!callback.requires_live_premium());
        Ok(())
    }
}
