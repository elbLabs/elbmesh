#![allow(dead_code)]

use std::time::Duration;

use elbmesh_core::{RestateOperationJournalObject, RestateOperationJournalObjectImpl};
use restate_sdk::prelude::{Endpoint, HttpServer};
use serde_json::json;
use tokio::net::TcpListener;

pub const RESTATE_URL_ENV: &str = "ELBMESH_RESTATE_URL";
pub const RESTATE_ADMIN_URL_ENV: &str = "ELBMESH_RESTATE_ADMIN_URL";
pub const RESTATE_SERVICE_ADVERTISE_HOST_ENV: &str = "ELBMESH_RESTATE_SERVICE_ADVERTISE_HOST";
pub const RESTATE_SERVICE_BIND_HOST_ENV: &str = "ELBMESH_RESTATE_SERVICE_BIND_HOST";
#[allow(dead_code)]
pub const RESTATE_DOCKER_SERVICE: &str = "restate";
#[allow(dead_code)]
pub const RESTATE_DOCKER_URL: &str = "http://127.0.0.1:8080";
#[allow(dead_code)]
pub const RESTATE_DOCKER_ADMIN_URL: &str = "http://127.0.0.1:9070";
pub const DEFAULT_RESTATE_ADMIN_URL: &str = RESTATE_DOCKER_ADMIN_URL;
pub const DEFAULT_RESTATE_SERVICE_ADVERTISE_HOST: &str = "127.0.0.1";
pub const DEFAULT_RESTATE_SERVICE_BIND_HOST: &str = "0.0.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestateHarnessConfig {
    url: String,
    admin_url: String,
    service_advertise_host: String,
    service_bind_host: String,
}

impl RestateHarnessConfig {
    pub fn from_env() -> Result<Self, RestateHarnessSkip> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup<F>(mut lookup: F) -> Result<Self, RestateHarnessSkip>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let Some(url) = lookup(RESTATE_URL_ENV).filter(|value| !value.trim().is_empty()) else {
            return Err(RestateHarnessSkip {
                reason: format!("{RESTATE_URL_ENV} is not set; skipping Restate integration test"),
            });
        };

        Ok(Self {
            url: trim_trailing_slashes(url),
            admin_url: lookup(RESTATE_ADMIN_URL_ENV)
                .filter(|value| !value.trim().is_empty())
                .map(trim_trailing_slashes)
                .unwrap_or_else(|| DEFAULT_RESTATE_ADMIN_URL.to_string()),
            service_advertise_host: lookup(RESTATE_SERVICE_ADVERTISE_HOST_ENV)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_RESTATE_SERVICE_ADVERTISE_HOST.to_string()),
            service_bind_host: lookup(RESTATE_SERVICE_BIND_HOST_ENV)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_RESTATE_SERVICE_BIND_HOST.to_string()),
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn admin_url(&self) -> &str {
        &self.admin_url
    }

    pub fn service_advertise_host(&self) -> &str {
        &self.service_advertise_host
    }

    pub fn service_bind_host(&self) -> &str {
        &self.service_bind_host
    }

    pub async fn start_operation_journal_endpoint(
        &self,
    ) -> Result<RestateLiveEndpoint, RestateLiveEndpointError> {
        let listener = TcpListener::bind(format!("{}:0", self.service_bind_host))
            .await
            .map_err(|source| RestateLiveEndpointError::Bind {
                address: format!("{}:0", self.service_bind_host),
                reason: source.to_string(),
            })?;
        let port = listener
            .local_addr()
            .map_err(|source| RestateLiveEndpointError::Bind {
                address: format!("{}:0", self.service_bind_host),
                reason: source.to_string(),
            })?
            .port();
        let service_url = format!("http://{}:{port}", self.service_advertise_host);
        let endpoint = Endpoint::builder()
            .bind(RestateOperationJournalObjectImpl.serve())
            .build();
        let handle = tokio::spawn(async move {
            HttpServer::new(endpoint)
                .serve_with_cancel(listener, std::future::pending::<()>())
                .await;
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        register_deployment(self.admin_url(), &service_url).await?;

        Ok(RestateLiveEndpoint {
            service_url,
            handle,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestateHarnessSkip {
    reason: String,
}

impl RestateHarnessSkip {
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

pub struct RestateLiveEndpoint {
    service_url: String,
    handle: tokio::task::JoinHandle<()>,
}

impl RestateLiveEndpoint {
    #[allow(dead_code)]
    pub fn service_url(&self) -> &str {
        &self.service_url
    }
}

impl Drop for RestateLiveEndpoint {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RestateLiveEndpointError {
    #[error("failed to bind Restate SDK endpoint at '{address}': {reason}")]
    Bind { address: String, reason: String },

    #[error(
        "failed to register Restate SDK endpoint '{service_url}' at admin '{admin_url}': {reason}"
    )]
    Register {
        admin_url: String,
        service_url: String,
        reason: String,
    },
}

async fn register_deployment(
    admin_url: &str,
    service_url: &str,
) -> Result<(), RestateLiveEndpointError> {
    let url = format!("{admin_url}/deployments");
    let response = reqwest::Client::new()
        .post(&url)
        .json(&json!({
            "uri": service_url,
            "force": true,
        }))
        .send()
        .await
        .map_err(|source| RestateLiveEndpointError::Register {
            admin_url: admin_url.to_string(),
            service_url: service_url.to_string(),
            reason: source.to_string(),
        })?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|source| format!("failed to read Restate admin error response: {source}"));
    Err(RestateLiveEndpointError::Register {
        admin_url: admin_url.to_string(),
        service_url: service_url.to_string(),
        reason: format!("HTTP {status}: {body}"),
    })
}

fn trim_trailing_slashes(mut value: String) -> String {
    while value.ends_with('/') {
        value.pop();
    }

    value
}
