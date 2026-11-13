//! A last-port policy over completed arrivals; it grants no movement or payment authority.
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TravelReturnRequirement {
    /// Select the latest completed receipt from this entire group, not just `return_from`.
    pub inbound_routes: Vec<String>,
    pub return_from: String,
}

impl TravelReturnRequirement {
    pub fn is_valid(&self) -> bool {
        if self.inbound_routes.is_empty()
            || self.inbound_routes.len() > 64
            || !self.inbound_routes.contains(&self.return_from)
        {
            return false;
        }
        let mut unique = std::collections::BTreeSet::new();
        self.inbound_routes
            .iter()
            .all(|route| receipt_route(route) && unique.insert(route))
    }

    /// `latest` is a completed arrival selected within `inbound_routes` by the fenced owner.
    /// Missing history, another port, ambiguous history and malformed policies stay closed.
    pub fn permits_latest_completed(&self, latest: Option<&str>) -> bool {
        self.is_valid() && latest == Some(self.return_from.as_str())
    }
}

// Keep the existing receipt grammar in migration 0223_npc_paid_travel.sql:12.
fn receipt_route(route: &str) -> bool {
    let Some((service, destination)) = route
        .strip_prefix("oteryn:service.travel.")
        .and_then(|key| key.split_once('/'))
    else {
        return false;
    };
    [service, destination].into_iter().all(|part| {
        !part.is_empty()
            && part.len() <= 80
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> TravelReturnRequirement {
        TravelReturnRequirement {
            inbound_routes: vec![
                "oteryn:service.travel.port_a/travora".into(),
                "oteryn:service.travel.port_b/travora".into(),
            ],
            return_from: "oteryn:service.travel.port_a/travora".into(),
        }
    }

    #[test]
    fn m38_travel_return_accepts_only_the_exact_recorded_port() {
        let policy = policy();
        assert!(policy.permits_latest_completed(Some("oteryn:service.travel.port_a/travora")));
        for latest in [
            None,
            Some("oteryn:service.travel.port_b/travora"),
            Some("oteryn:service.travel.port_a/travora_extra"),
            Some("oteryn:service.travel.PORT_A/travora"),
        ] {
            assert!(!policy.permits_latest_completed(latest));
        }
    }

    #[test]
    fn m38_travel_return_group_is_bounded_unique_and_contains_the_expected_port() {
        let mut policy = policy();
        assert!(policy.is_valid());
        policy.inbound_routes.reverse();
        assert!(policy.is_valid());
        policy.return_from = "oteryn:service.travel.unknown/travora".into();
        assert!(!policy.is_valid());
        let mut policy = super::tests::policy();
        policy.inbound_routes.clear();
        assert!(!policy.is_valid());
        policy.inbound_routes = (0..64)
            .map(|i| format!("oteryn:service.travel.port_{i}/travora"))
            .collect();
        policy.return_from = policy.inbound_routes[0].clone();
        assert!(policy.is_valid());
        policy
            .inbound_routes
            .push("oteryn:service.travel.extra/travora".into());
        assert!(!policy.is_valid());
        policy.inbound_routes = vec![policy.return_from.clone(), policy.return_from.clone()];
        assert!(!policy.is_valid());
    }

    #[test]
    fn m38_travel_return_rejects_noncanonical_receipt_routes_without_normalizing() {
        for invalid in [
            "",
            "port_a/travora",
            "oteryn:service.travel.Port_a/travora",
            "oteryn:service.travel.port_a/travora/extra",
            "oteryn:service.travel.port_a/travora ",
            "oteryn:service.travel.port_a/",
            "oteryn:service.travel./travora",
        ] {
            let policy = TravelReturnRequirement {
                inbound_routes: vec![invalid.into()],
                return_from: invalid.into(),
            };
            assert!(!policy.permits_latest_completed(Some(invalid)));
        }
    }

    #[test]
    fn m38_travel_return_rejects_implicit_fallback_fields() {
        assert!(
            serde_json::from_value::<TravelReturnRequirement>(serde_json::json!({
                "inbound_routes":["oteryn:service.travel.port_a/travora"],
                "return_from":"oteryn:service.travel.port_a/travora", "fallback":"any_port"
            }))
            .is_err()
        );
    }
}
