use crate::config::TwitchConfig;
use crate::scoreboard::{Scoreboard, StreamerStalkingState};
use crate::twitch_listener::TwitchListener;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use std::env;
use std::time::Duration;
use axum::http::Method;
use tokio::time::interval;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;

mod twitch_listener;
mod config;
mod scoreboard;

#[derive(Clone)]
struct AppState {
    scoreboard: Scoreboard,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let twitch_config = TwitchConfig::from_env().expect("failed to load config");
    let twitch = TwitchListener::from_config(&twitch_config)
        .await
        .expect("failed to initialize Twitch listener");
    let twitch = twitch.launch();
    let scoreboard = Scoreboard::new();

    {
        let scoreboard = scoreboard.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let mut last_update_timestamp = std::time::Instant::now();
            loop {
                interval.tick().await;
                let state = {
                    twitch.lock().await.get_status()
                };
                if !state.initialized {
                    continue;
                }

                if state.last_update > last_update_timestamp {
                    last_update_timestamp = state.last_update;
                    scoreboard.update_new(&state.online_status);
                }
            }
        });
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET])
        .allow_headers(Any);
    let static_files = ServeDir::new("static");
    let app_state = AppState { scoreboard };
    let app = Router::new()
        .route("/health", get(|| async { "At least http server is still alive!" }))
        .route("/status", get(get_status))
        .fallback_service(static_files)
        .layer(cors)
        .with_state(app_state);

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    info!("Starting server on {}", bind_addr);
    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}


async fn get_status(
    State(state): State<AppState>,
) -> Json<StreamerStalkingState> {
    let state = state.scoreboard.get_state();
    Json(state)
}
