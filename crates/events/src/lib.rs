//! Transport-neutral event envelope and bus interfaces.

use std::{future::Future, pin::Pin};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type EventFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub id: String,
    pub topic: String,
    pub version: u32,
    pub producer: String,
    pub occurred_at: String,
    pub correlation_id: Option<String>,
    pub payload: serde_json::Value,
}

impl EventEnvelope {
    pub fn validate(&self) -> Result<(), EventError> {
        if !valid_token(&self.id, 128) {
            return Err(EventError::Invalid("id".into()));
        }
        if !valid_topic(&self.topic) || !self.topic.starts_with(&format!("{}.", self.producer)) {
            return Err(EventError::Invalid("topic".into()));
        }
        if self.version == 0 || !valid_plugin_id(&self.producer) {
            return Err(EventError::Invalid("version/producer".into()));
        }
        if self.occurred_at.len() < 20
            || self.occurred_at.len() > 64
            || !self.occurred_at.ends_with('Z')
            || self.occurred_at.chars().any(char::is_control)
        {
            return Err(EventError::Invalid("occurred_at".into()));
        }
        if self
            .correlation_id
            .as_deref()
            .is_some_and(|value| !valid_token(value, 128))
        {
            return Err(EventError::Invalid("correlation_id".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventSubscription {
    pub consumer: String,
    pub topic: String,
    pub version_requirement: String,
    pub handler: String,
    pub delivery: DeliveryGuarantee,
}

impl EventSubscription {
    pub fn validate(&self) -> Result<(), EventError> {
        if !valid_plugin_id(&self.consumer)
            || !valid_topic(&self.topic)
            || !valid_plugin_id(&self.handler)
            || semver::VersionReq::parse(&self.version_requirement).is_err()
        {
            return Err(EventError::Invalid("subscription".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryGuarantee {
    AtMostOnce,
    AtLeastOnce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDisposition {
    Acknowledge,
    Retry,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EventError {
    #[error("event validation failed: {0}")]
    Invalid(String),
    #[error("event transport failed: {0}")]
    Transport(String),
    #[error("event handler failed: {0}")]
    Handler(String),
}

pub trait EventPublisher: Send + Sync {
    fn publish<'a>(&'a self, event: &'a EventEnvelope) -> EventFuture<'a, Result<(), EventError>>;
}

pub trait EventHandler: Send + Sync {
    fn handle<'a>(
        &'a self,
        event: &'a EventEnvelope,
    ) -> EventFuture<'a, Result<EventDisposition, EventError>>;
}

pub trait EventBus: EventPublisher {
    fn subscribe(
        &self,
        subscription: EventSubscription,
        handler: Box<dyn EventHandler>,
    ) -> Result<(), EventError>;
}

fn valid_topic(value: &str) -> bool {
    value.len() <= 128 && value.split('.').count() >= 2 && value.split('.').all(valid_plugin_id)
}

fn valid_plugin_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.contains("--")
}

fn valid_token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> EventEnvelope {
        EventEnvelope {
            id: "01event".into(),
            topic: "photo-backup.asset-created".into(),
            version: 1,
            producer: "photo-backup".into(),
            occurred_at: "2026-08-27T12:00:00Z".into(),
            correlation_id: Some("request-1".into()),
            payload: serde_json::json!({"asset_id": "asset-1"}),
        }
    }

    #[test]
    fn valid_envelope_is_transport_neutral_json() {
        let event = event();
        event.validate().unwrap();
        let encoded = serde_json::to_vec(&event).unwrap();
        assert_eq!(
            serde_json::from_slice::<EventEnvelope>(&encoded).unwrap(),
            event
        );
    }

    #[test]
    fn malformed_topics_and_versions_are_rejected() {
        let mut event = event();
        event.topic = "foreign/path".into();
        assert!(event.validate().is_err());
        event.topic = "photo-backup.asset-created".into();
        event.version = 0;
        assert!(event.validate().is_err());
    }

    #[test]
    fn producer_owns_topic_and_subscription_range_is_semantic() {
        let mut event = event();
        event.producer = "sentinel-monitor".into();
        assert!(event.validate().is_err());

        let subscription = EventSubscription {
            consumer: "audit-module".into(),
            topic: "photo-backup.asset-created".into(),
            version_requirement: ">=1.0.0, <2.0.0".into(),
            handler: "index-asset".into(),
            delivery: DeliveryGuarantee::AtLeastOnce,
        };
        subscription.validate().unwrap();
    }
}
