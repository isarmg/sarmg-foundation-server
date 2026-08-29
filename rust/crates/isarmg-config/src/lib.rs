use anyhow::Context;
use serde::de::DeserializeOwned;

pub fn load_toml<T: DeserializeOwned>(path: &std::path::Path) -> anyhow::Result<T> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

pub fn require_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("{name} is required"))
}
