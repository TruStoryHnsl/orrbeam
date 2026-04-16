//! Read/write helper for the Sunshine `sunshine.conf` configuration file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when reading or writing `sunshine.conf`.
#[derive(Error, Debug)]
pub enum SunshineConfError {
    /// An I/O error while reading or writing the config file.
    #[error("failed to read sunshine.conf: {0}")]
    Read(#[from] std::io::Error),
    /// The `sunshine.conf` file does not exist at the expected path.
    #[error("sunshine.conf not found")]
    NotFound,
}

/// Sunshine streaming settings exposed in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SunshineSettings {
    /// Display output name (e.g. `"DP-1"`, `"HDMI-1"`).
    pub output_name: Option<String>,
    /// Target frame rate in frames per second.
    pub fps: Option<u32>,
    /// Target bitrate in kilobits per second.
    pub bitrate: Option<u32>,
    /// Hardware encoder to use (e.g. `"nvenc"`, `"vaapi"`, `"videotoolbox"`).
    pub encoder: Option<String>,
    /// Video codec (e.g. `"h264"`, `"h265"`, `"av1"`).
    pub codec: Option<String>,
    /// Number of audio channels.
    pub channels: Option<u32>,
}

impl SunshineSettings {
    /// Build from raw key-value map.
    pub fn from_conf(conf: &HashMap<String, String>) -> Self {
        Self {
            output_name: conf.get("output_name").cloned(),
            fps: conf.get("fps").and_then(|v| v.parse().ok()),
            bitrate: conf.get("bitrate_in_kbits").and_then(|v| v.parse().ok()),
            encoder: conf.get("encoder").cloned(),
            codec: conf.get("codec").cloned(),
            channels: conf.get("channels").and_then(|v| v.parse().ok()),
        }
    }

    /// Convert back to key-value pairs (only set fields).
    pub fn to_conf(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(v) = &self.output_name {
            map.insert("output_name".into(), v.clone());
        }
        if let Some(v) = self.fps {
            map.insert("fps".into(), v.to_string());
        }
        if let Some(v) = self.bitrate {
            map.insert("bitrate_in_kbits".into(), v.to_string());
        }
        if let Some(v) = &self.encoder {
            map.insert("encoder".into(), v.clone());
        }
        if let Some(v) = &self.codec {
            map.insert("codec".into(), v.clone());
        }
        if let Some(v) = self.channels {
            map.insert("channels".into(), v.to_string());
        }
        map
    }
}

/// Platform-appropriate sunshine.conf path.
pub fn conf_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let app_support = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Sunshine")
            .join("sunshine.conf");
        if app_support.exists() {
            return app_support;
        }
    }

    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sunshine")
        .join("sunshine.conf")
}

/// Parse sunshine.conf into key=value pairs.
pub fn read_conf() -> Result<HashMap<String, String>, SunshineConfError> {
    let path = conf_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&path)?;
    let mut config = HashMap::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            config.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(config)
}

/// Write config updates to sunshine.conf, preserving comments and ordering.
pub fn write_conf(updates: &HashMap<String, String>) -> Result<(), SunshineConfError> {
    let path = conf_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let existing = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let mut written_keys = std::collections::HashSet::new();
    let mut new_lines = Vec::new();

    for line in existing.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && let Some((key, _)) = trimmed.split_once('=')
        {
            let key = key.trim();
            if let Some(value) = updates.get(key) {
                new_lines.push(format!("{key} = {value}"));
                written_keys.insert(key.to_string());
                continue;
            }
        }
        new_lines.push(line.to_string());
    }

    // Append new keys not in the original file
    for (key, value) in updates {
        if !written_keys.contains(key.as_str()) {
            new_lines.push(format!("{key} = {value}"));
        }
    }

    std::fs::write(&path, new_lines.join("\n") + "\n")?;
    Ok(())
}

/// Read the current Sunshine settings.
pub fn get_settings() -> Result<SunshineSettings, SunshineConfError> {
    let conf = read_conf()?;
    Ok(SunshineSettings::from_conf(&conf))
}

/// Update Sunshine settings (merges with existing config).
pub fn set_settings(settings: &SunshineSettings) -> Result<(), SunshineConfError> {
    let updates = settings.to_conf();
    write_conf(&updates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settings() -> SunshineSettings {
        SunshineSettings {
            output_name: Some("DP-1".to_string()),
            fps: Some(60),
            bitrate: Some(20000),
            encoder: Some("nvenc".to_string()),
            codec: Some("h265".to_string()),
            channels: Some(2),
        }
    }

    #[test]
    fn settings_from_conf_parses_known_keys() {
        let mut conf = HashMap::new();
        conf.insert("output_name".to_string(), "HDMI-1".to_string());
        conf.insert("fps".to_string(), "120".to_string());
        conf.insert("bitrate_in_kbits".to_string(), "50000".to_string());
        conf.insert("encoder".to_string(), "vaapi".to_string());
        conf.insert("codec".to_string(), "h264".to_string());
        conf.insert("channels".to_string(), "2".to_string());

        let s = SunshineSettings::from_conf(&conf);
        assert_eq!(s.output_name.as_deref(), Some("HDMI-1"));
        assert_eq!(s.fps, Some(120));
        assert_eq!(s.bitrate, Some(50000));
        assert_eq!(s.encoder.as_deref(), Some("vaapi"));
        assert_eq!(s.codec.as_deref(), Some("h264"));
        assert_eq!(s.channels, Some(2));
    }

    #[test]
    fn settings_from_conf_ignores_unknown_keys() {
        let mut conf = HashMap::new();
        conf.insert("unknown_key".to_string(), "value".to_string());

        let s = SunshineSettings::from_conf(&conf);
        assert!(s.output_name.is_none());
        assert!(s.fps.is_none());
    }

    #[test]
    fn to_conf_roundtrip() {
        let s = make_settings();
        let conf = s.to_conf();
        let recovered = SunshineSettings::from_conf(&conf);
        assert_eq!(recovered.output_name, s.output_name);
        assert_eq!(recovered.fps, s.fps);
        assert_eq!(recovered.bitrate, s.bitrate);
        assert_eq!(recovered.encoder, s.encoder);
        assert_eq!(recovered.codec, s.codec);
        assert_eq!(recovered.channels, s.channels);
    }

    #[test]
    fn to_conf_omits_none_fields() {
        let s = SunshineSettings {
            output_name: None,
            fps: Some(30),
            bitrate: None,
            encoder: None,
            codec: None,
            channels: None,
        };
        let conf = s.to_conf();
        assert!(
            !conf.contains_key("output_name"),
            "none fields must not appear"
        );
        assert!(conf.contains_key("fps"));
    }
}
