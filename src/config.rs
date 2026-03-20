use std::env;
use std::time::Duration;
use anyhow::Context;

#[derive(Clone, Debug)]
pub struct TwitchConfig {
    pub client_id: String,
    pub client_secret: String,
    pub target_login: String,
    pub poll_interval: Duration,
}

impl TwitchConfig {
    pub fn from_env() -> anyhow::Result<TwitchConfig> {
        let client_id = env::var("TWITCH_CLIENT_ID").context("TWITCH_CLIENT_ID not set")?;
        let client_secret = env::var("TWITCH_CLIENT_SECRET").context("TWITCH_CLIENT_SECRET not set")?;
        let target_login = env::var("TARGET_LOGIN").context("TARGET_LOGIN not set")?;
        let poll_interval_secs: u64 = env::var("POLL_INTERVAL_SECS")
            .unwrap_or_else(|_| "5".into())
            .parse()
            .context("POLL_INTERVAL_SECS must be a number")?;

        Ok(TwitchConfig {
            client_id,
            client_secret,
            target_login,
            poll_interval: Duration::from_secs(poll_interval_secs),
        })
    }
}