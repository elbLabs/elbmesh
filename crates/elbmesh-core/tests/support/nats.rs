pub const NATS_URL_ENV: &str = "ELBMESH_NATS_URL";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsHarnessConfig {
    url: String,
}

impl NatsHarnessConfig {
    pub fn from_env() -> Result<Self, NatsHarnessSkip> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, NatsHarnessSkip>
    where
        F: FnOnce(&str) -> Option<String>,
    {
        match lookup(NATS_URL_ENV) {
            Some(url) if !url.trim().is_empty() => Ok(Self { url }),
            _ => Err(NatsHarnessSkip {
                reason: format!("{NATS_URL_ENV} is not set; skipping NATS integration test"),
            }),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsHarnessSkip {
    reason: String,
}

impl NatsHarnessSkip {
    pub fn reason(&self) -> &str {
        &self.reason
    }
}
