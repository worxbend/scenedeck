//! Readiness plans belong to the actual connection, independently of collection.
use super::AppConfig;
use crate::domain::preflight::PreflightPlan;

impl AppConfig {
    pub(crate) fn preflight_plan(
        &self,
        connection: &str,
        endpoint: &str,
    ) -> Option<&PreflightPlan> {
        let mut matches = self
            .preflight_plans
            .iter()
            .filter(|saved| saved.connection == connection && saved.endpoint == endpoint);
        let saved = matches.next()?;
        matches.next().is_none().then_some(&saved.plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::preflight::SavedPreflight;
    #[test]
    fn plans_round_trip_with_connection_and_endpoint_isolation() {
        let mut config: AppConfig = serde_json::from_str("{}").unwrap();
        assert!(config.preflight_plans.is_empty());
        let plan = PreflightPlan {
            expected_collection: "Podcast".into(),
            manual_items: vec!["Guests ready".into()],
            ..Default::default()
        };
        config.preflight_plans.push(SavedPreflight {
            connection: "one".into(),
            endpoint: "ws://studio:4455".into(),
            plan: plan.clone(),
        });
        let encoded = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&encoded).unwrap();
        assert_eq!(
            parsed.preflight_plan("one", "ws://studio:4455"),
            Some(&plan)
        );
        assert!(parsed.preflight_plan("two", "ws://studio:4455").is_none());
        assert!(parsed.preflight_plan("one", "ws://laptop:4455").is_none());
        config
            .preflight_plans
            .push(config.preflight_plans[0].clone());
        assert!(
            config.preflight_plan("one", "ws://studio:4455").is_none(),
            "ambiguous plans must not silently select one"
        );
    }
}
