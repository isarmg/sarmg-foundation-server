use super::*;

pub const ADMINISTRATORS_PATH: &str = "/api/v2/platform/administrators";

// Deliberately no Debug implementation: these requests contain credentials.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdministratorCreateRequest {
    #[serde(deserialize_with = "deserialize_administrator_username_candidate")]
    pub username: String,
    #[serde(deserialize_with = "deserialize_secret_text")]
    pub password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdministratorPasswordRequest {
    #[serde(deserialize_with = "deserialize_secret_text")]
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdministratorSummary {
    #[serde(deserialize_with = "deserialize_identifier")]
    pub administrator_id: String,
    #[serde(deserialize_with = "deserialize_canonical_administrator_username")]
    pub username: String,
    pub active: bool,
    #[serde(deserialize_with = "deserialize_non_negative_safe_integer")]
    pub created_at_micros: u64,
    #[serde(deserialize_with = "deserialize_non_negative_safe_integer")]
    pub updated_at_micros: u64,
    #[serde(deserialize_with = "optional_timestamp")]
    pub last_login_at_micros: Option<u64>,
}

impl Serialize for AdministratorSummary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        validate_identifier("administrator_id", &self.administrator_id)
            .map_err(S::Error::custom)?;
        require_canonical_administrator_username(&self.username).map_err(S::Error::custom)?;
        for (name, timestamp) in [
            ("created_at_micros", Some(self.created_at_micros)),
            ("updated_at_micros", Some(self.updated_at_micros)),
            ("last_login_at_micros", self.last_login_at_micros),
        ] {
            if let Some(value) = timestamp {
                validate_non_negative_safe_integer(name, value).map_err(S::Error::custom)?;
            }
        }
        let mut state = serializer.serialize_struct("AdministratorSummary", 6)?;
        state.serialize_field("administrator_id", &self.administrator_id)?;
        state.serialize_field("username", &self.username)?;
        state.serialize_field("active", &self.active)?;
        state.serialize_field("created_at_micros", &self.created_at_micros)?;
        state.serialize_field("updated_at_micros", &self.updated_at_micros)?;
        state.serialize_field("last_login_at_micros", &self.last_login_at_micros)?;
        state.end()
    }
}

fn optional_timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    #[derive(Deserialize)]
    struct Timestamp(#[serde(deserialize_with = "deserialize_non_negative_safe_integer")] u64);
    Option::<Timestamp>::deserialize(deserializer).map(|value| value.map(|value| value.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn management_matches_shared_typescript_and_schema_fixtures() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/contracts/fixtures/administrator-management.fixtures.json"
        ))
        .unwrap();
        for (group, expected) in [("valid", true), ("invalid", false)] {
            for value in fixtures[group].as_array().unwrap() {
                let accepted = serde_json::from_value::<AdministratorCreateRequest>(value.clone())
                    .is_ok()
                    || serde_json::from_value::<AdministratorPasswordRequest>(value.clone())
                        .is_ok()
                    || serde_json::from_value::<AdministratorSummary>(value.clone()).is_ok();
                assert_eq!(accepted, expected, "fixture: {value}");
            }
        }
    }
    #[test]
    fn summary_rejects_secret_fields_and_unsafe_timestamps() {
        let mut value = serde_json::json!({"administrator_id": "A".repeat(43), "username": "admin", "active": true,
            "created_at_micros": 1, "updated_at_micros": 2, "last_login_at_micros": null});
        let mut summary: AdministratorSummary = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&summary).unwrap(), value);
        value["password_hash"] = "secret".into();
        assert!(serde_json::from_value::<AdministratorSummary>(value.clone()).is_err());
        value.as_object_mut().unwrap().remove("password_hash");
        for key in [
            "created_at_micros",
            "updated_at_micros",
            "last_login_at_micros",
        ] {
            let mut unsafe_value = value.clone();
            unsafe_value[key] = (MAX_SAFE_JSON_INTEGER + 1).into();
            assert!(serde_json::from_value::<AdministratorSummary>(unsafe_value).is_err());
        }
        value
            .as_object_mut()
            .unwrap()
            .remove("last_login_at_micros");
        assert!(serde_json::from_value::<AdministratorSummary>(value).is_err());
        summary.created_at_micros = MAX_SAFE_JSON_INTEGER + 1;
        assert!(serde_json::to_value(summary).is_err());
    }
}
