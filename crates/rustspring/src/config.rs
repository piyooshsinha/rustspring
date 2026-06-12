//! Profile-aware configuration, modeled on Spring Boot's `application-{profile}.yml`.
//!
//! Resolution order (later wins):
//!   1. `config/application.toml`           — defaults for every profile
//!   2. `config/application-{profile}.toml` — profile overrides
//!   3. Environment variables prefixed `APP_` (e.g. `APP_SERVER__PORT=9090`)
//!
//! The active profile comes from `APP_PROFILE` (default: `dev`),
//! the equivalent of `spring.profiles.active`.

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    10
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StaticConfig {
    /// Directory of built frontend assets (e.g. `frontend/dist`).
    /// When set, unmatched routes serve the SPA with an `index.html` fallback.
    pub dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    pub database: Option<DatabaseConfig>,
    #[serde(rename = "static", default)]
    pub static_files: StaticConfig,
    #[serde(skip)]
    pub profile: String,
}

/// The loaded configuration plus the raw [`Figment`], so applications can
/// pull their own custom sections with [`ConfigSource::section`].
#[derive(Clone)]
pub struct ConfigSource {
    pub app: AppConfig,
    figment: Figment,
}

impl ConfigSource {
    pub fn load() -> Result<Self, figment::Error> {
        let profile = std::env::var("APP_PROFILE").unwrap_or_else(|_| "dev".to_string());

        let figment = Figment::new()
            .merge(Toml::file("config/application.toml"))
            .merge(Toml::file(format!("config/application-{profile}.toml")))
            .merge(Env::prefixed("APP_").split("__"));

        let mut app: AppConfig = figment.extract()?;
        app.profile = profile;

        Ok(Self { app, figment })
    }

    /// Extract a custom config section into your own struct, like
    /// `@ConfigurationProperties(prefix = "greeting")`:
    ///
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct GreetingConfig { template: String }
    /// let greeting: GreetingConfig = cfg.section("greeting")?;
    /// ```
    pub fn section<'de, T: Deserialize<'de>>(&self, key: &str) -> Result<T, figment::Error> {
        self.figment.extract_inner(key)
    }
}
