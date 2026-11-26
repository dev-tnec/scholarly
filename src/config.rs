use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind: String,
    pub port: u16,
    pub base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;
        Ok(Self {
            database_url,
            bind: env::var("BIND").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            base_url: env::var("BASE_URL")
                .unwrap_or_else(|_| "https://scholarly.titiknolenglish.com".into())
                .trim_end_matches('/')
                .to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("DATABASE_URL is not set")]
    MissingDatabaseUrl,
}

#[cfg(test)]
mod tests {
    #[test]
    fn base_url_has_no_trailing_slash() {
        // Config::from_env needs a live env; test the normalization directly.
        let raw = "https://example.test/";
        assert_eq!(raw.trim_end_matches('/'), "https://example.test");
    }

    #[test]
    fn port_parsing_falls_back_on_garbage() {
        assert_eq!("not-a-port".parse::<u16>().ok().unwrap_or(8080), 8080);
        assert_eq!("9090".parse::<u16>().ok().unwrap_or(8080), 9090);
    }
}
