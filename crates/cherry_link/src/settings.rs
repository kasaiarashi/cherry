use crate::protocol::BuildConfiguration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Settings for CherryLink Unreal Engine integration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CherryLinkSettings {
    /// Enable CherryLink integration for Unreal Engine projects
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// TCP port for CherryLink connection (default: 21567)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Automatically connect when an Unreal Engine project is detected
    #[serde(default = "default_auto_connect")]
    pub auto_connect: bool,

    /// Default build configuration
    #[serde(default)]
    pub default_configuration: BuildConfiguration,
}

fn default_enabled() -> bool {
    true
}

fn default_port() -> u16 {
    21567
}

fn default_auto_connect() -> bool {
    true
}

impl CherryLinkSettings {
    /// Get default settings
    pub fn default_settings() -> Self {
        Self {
            enabled: default_enabled(),
            port: default_port(),
            auto_connect: default_auto_connect(),
            default_configuration: BuildConfiguration::default(),
        }
    }
}
