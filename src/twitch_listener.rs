use anyhow::{anyhow, Context};
use std::sync::Arc;
use std::time::Instant;
use crate::config::TwitchConfig;
use tokio::sync::Mutex;
use tracing::{info, warn};
use twitch_api::helix::streams::{get_streams, Stream};
use twitch_api::helix::HelixClient;
use twitch_api::twitch_oauth2::{AppAccessToken, ClientId, ClientSecret, TwitchToken};
use twitch_api::types::UserId;


#[derive(Debug, Clone)]
pub enum OnlineStatus {
    Offline,
    Live(Stream),
}

#[derive(Debug, Clone)]
pub struct TwitchListenerStatus {
    pub initialized: bool,
    pub online_status: OnlineStatus,
    pub last_update: Instant,
}

pub struct TwitchListener {
    state: TwitchListenerStatus,
    config: TwitchConfig,
    token: Arc<Mutex<AppAccessToken>>,
    helix: HelixClient<'static, reqwest::Client>,
    user_id: UserId,
}

impl TwitchListener {
    pub async fn from_config(config: &TwitchConfig) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder().build()?;
        let helix: HelixClient<reqwest::Client> = HelixClient::with_client(http);

        let client_id = ClientId::new(config.client_id.clone());
        let client_secret = ClientSecret::new(config.client_secret.clone());

        let token = AppAccessToken::get_app_access_token(
            &helix.get_client().clone(),
            client_id.clone(),
            client_secret.clone(),
            vec![],
        ).await?;

        let user_id = helix
            .get_user_from_login(&config.target_login, &token)
            .await?
            .map(|u| u.id)
            .ok_or_else(|| anyhow!("user '{}' not found", config.target_login))?;
        info!("User {} resolved to ID {user_id}", config.target_login);

        let temporal_state = TwitchListenerStatus {
            initialized: false,
            online_status: OnlineStatus::Offline,
            last_update: Instant::now(),
        };

        Ok(Self {
            state: temporal_state,
            helix,
            config: config.clone(),
            token: Arc::new(Mutex::new(token)),
            user_id,
        })
    }

    async fn ensure_token_valid(&self) -> anyhow::Result<()> {
        let mut token = self.token.lock().await;
        if token.is_elapsed() {
            let client_id = ClientId::new(self.config.client_id.clone());
            let client_secret = ClientSecret::new(self.config.client_secret.clone());

            *token = AppAccessToken::get_app_access_token(
                &self.helix.get_client().clone(),
                client_id,
                client_secret,
                vec![],
            )
                .await
                .context("failed to refresh app access token")?;
        }
        Ok(())
    }

    pub fn launch(self: Self) -> Arc<Mutex<TwitchListener>> {
        let _self = Arc::new(Mutex::new(self));
        {
            let _self = Arc::clone(&_self);
            tokio::spawn(async move {
                loop {
                    let wait_time = {
                        let mut _self = _self.lock().await;
                        _self.tick().await;
                        _self.config.poll_interval
                    };
                    tokio::time::sleep(wait_time).await;
                }
            });
        }
        _self
    }

    async fn get_online_status(&self) -> anyhow::Result<Option<Stream>> {
        let token = self.token.lock().await.clone();
        let user_id = self.user_id.clone();

        let req = get_streams::GetStreamsRequest::user_ids(vec![user_id]);
        let response = self.helix.req_get(req, &token).await?;
        Ok(response.data.first().map(|it| it.clone()))
    }

    pub fn get_status(&self) -> TwitchListenerStatus {
        self.state.clone()
    }

    async fn tick(&mut self) {
        let err = self.ensure_token_valid().await;
        if let Err(e) = err {
            warn!("Failed to ensure valid token: {e}");
            return;
        }

        let update_time = Instant::now();
        let online_status = self.get_online_status().await;
        let online_status = match online_status {
            Ok(status) => status,
            Err(e) => {
                warn!("Failed to get stream status: {e}");
                return;
            }
        };

        let new_status = match (online_status) {
            Some(stream) => OnlineStatus::Live(stream),
            None => OnlineStatus::Offline,
        };

        self.state = TwitchListenerStatus {
            initialized: true,
            online_status: new_status,
            last_update: update_time,
        };
    }
}
