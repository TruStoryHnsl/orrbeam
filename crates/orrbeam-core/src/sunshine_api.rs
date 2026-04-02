use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SunshineApiError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("PIN rejected by Sunshine")]
    PinRejected,
    #[error("Sunshine API not reachable")]
    Unreachable,
    #[error("missing Sunshine credentials")]
    NoCredentials,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinRequest {
    pin: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct PinResponse {
    status: Option<bool>,
}

/// Submit a pairing PIN to the local Sunshine instance.
///
/// Sunshine's web API runs on https://localhost:47990 with self-signed TLS.
/// The PIN endpoint only succeeds when a Moonlight pairing request is actively
/// pending, so we retry up to `max_attempts` times with 1-second intervals.
pub async fn submit_pin_local(
    username: &str,
    password: &str,
    pin: &str,
    client_name: &str,
    max_attempts: u32,
) -> Result<(), SunshineApiError> {
    if username.is_empty() || password.is_empty() {
        return Err(SunshineApiError::NoCredentials);
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| SunshineApiError::Http(e.to_string()))?;

    let url = "https://127.0.0.1:47990/api/pin";

    for attempt in 1..=max_attempts {
        match client
            .post(url)
            .basic_auth(username, Some(password))
            .json(&PinRequest {
                pin: pin.to_string(),
                name: client_name.to_string(),
            })
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(data) = resp.json::<PinResponse>().await {
                    if data.status.unwrap_or(false) {
                        tracing::info!("PIN accepted by local Sunshine (attempt {attempt})");
                        return Ok(());
                    }
                }
                tracing::debug!("PIN attempt {attempt}/{max_attempts}: not yet accepted");
            }
            Ok(resp) => {
                tracing::debug!(
                    "PIN attempt {attempt}/{max_attempts}: HTTP {}",
                    resp.status()
                );
            }
            Err(e) => {
                tracing::debug!("PIN attempt {attempt}/{max_attempts}: {e}");
            }
        }

        if attempt < max_attempts {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    Err(SunshineApiError::PinRejected)
}
