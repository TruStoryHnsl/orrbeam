//! `control_curl` — dev tool for sending a signed GET request to an orrbeam
//! control-plane endpoint.
//!
//! # Usage
//!
//! ```text
//! cargo run -p orrbeam-net --example control_curl -- --peer 192.168.1.132:47782 --path /v1/status
//! ```
//!
//! The local node identity is loaded from the default path
//! (`~/.local/share/orrbeam/identity/signing.key`).  TLS certificate
//! verification is disabled — this is intentional; the tool is for ad-hoc
//! debugging against peers that may not yet be in the trusted-peer store.

use std::process;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use reqwest::ClientBuilder;

use orrbeam_core::identity::Identity;
use orrbeam_core::wire::{
    sign_request, HEADER_KEY_ID, HEADER_NONCE, HEADER_SIGNATURE, HEADER_TIMESTAMP, HEADER_VERSION,
};

#[derive(Parser, Debug)]
#[command(
    name = "control_curl",
    about = "Send a signed GET request to an orrbeam control-plane endpoint"
)]
struct Args {
    /// Remote peer address and port, e.g. `192.168.1.132:47782`
    #[arg(long)]
    peer: String,

    /// Control-plane path, e.g. `/v1/status` or `/v1/hello`
    #[arg(long)]
    path: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Load or create local identity.
    let identity = match Identity::load_or_create() {
        Ok(id) => Arc::new(id),
        Err(e) => {
            eprintln!("error: failed to load identity: {e}");
            process::exit(1);
        }
    };

    // Build a reqwest client with TLS verification disabled.
    // SECURITY: this is intentional for a dev/debug tool only.
    let http = match ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to build HTTP client: {e}");
            process::exit(1);
        }
    };

    let url = format!("https://{}{}", args.peer, args.path);

    // Sign the request.
    let body_bytes: Vec<u8> = Vec::new();
    let headers = sign_request(&identity, "GET", &args.path, &body_bytes);

    let response = http
        .get(&url)
        .header(HEADER_VERSION, &headers.version)
        .header(HEADER_KEY_ID, &headers.key_id)
        .header(HEADER_TIMESTAMP, &headers.timestamp)
        .header(HEADER_NONCE, &headers.nonce)
        .header(HEADER_SIGNATURE, &headers.signature)
        .send()
        .await;

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: request failed: {e}");
            process::exit(1);
        }
    };

    let status = response.status();
    let body_text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: failed to read response body: {e}");
            process::exit(1);
        }
    };

    // Pretty-print JSON if possible, otherwise raw text.
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_text) {
        println!("HTTP {status}");
        println!("{}", serde_json::to_string_pretty(&json_val).unwrap_or(body_text));
    } else {
        println!("HTTP {status}");
        println!("{body_text}");
    }

    if !status.is_success() {
        process::exit(1);
    }
}
