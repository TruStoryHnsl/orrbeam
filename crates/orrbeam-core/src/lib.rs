//! Core types and utilities for the orrbeam mesh.
//!
//! This crate provides:
//! - [`Config`] — YAML-backed application configuration.
//! - [`Identity`] — Ed25519 node identity for mesh authentication.
//! - [`Node`] / [`NodeRegistry`] — mesh node representation and registry.
//! - [`peers`] — trusted peer store with fingerprint indexing.
//! - [`tls`] — self-signed TLS identity derived from the Ed25519 key.
//! - [`wire`] — control-plane wire protocol types and signing/verification.
//! - [`sunshine_api`] — HTTP client for the local Sunshine pairing API.
//! - [`sunshine_conf`] — read/write helper for `sunshine.conf`.

#![warn(missing_docs)]

pub mod config;
pub mod identity;
pub mod node;
pub mod peers;
pub mod secure_file;
pub mod sunshine_api;
pub mod sunshine_conf;
pub mod tls;
pub mod wire;

pub use config::Config;
pub use identity::Identity;
pub use node::{DiscoverySource, Node, NodeRegistry, NodeRegistryError, NodeState};
pub use sunshine_conf::SunshineSettings;
