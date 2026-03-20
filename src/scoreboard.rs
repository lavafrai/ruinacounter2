use std::path::PathBuf;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use crate::twitch_listener::OnlineStatus;

#[derive(Serialize, Deserialize, Debug)]
pub struct Highscore {
    pub duration: Duration,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StreamerStalkingState {
    pub last_detected_online: DateTime<Utc>,
    pub last_detected_offline: DateTime<Utc>,
    pub last_update_time: DateTime<Utc>,
    pub offline_highscore: Highscore,
    pub is_online: bool,
    pub ongoing: Option<OngoingStreamInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OngoingStreamInfo {
    pub title: String,
    pub game: String,
    pub started_at: DateTime<Utc>,
}

impl StreamerStalkingState {
    pub fn new() -> Self {
        let now = Utc::now();
        StreamerStalkingState {
            last_detected_online: now,
            last_detected_offline: now,
            last_update_time: now,
            offline_highscore: Highscore {
                duration: Duration::ZERO,
                start_time: now,
                end_time: now,
            },
            is_online: false,
            ongoing: None,
        }
    }

    pub fn restore_or_new(file: &PathBuf) -> Self {
        match std::fs::read_to_string(file) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self, file: &PathBuf) -> anyhow::Result<()> {
        self.save_atomic(file)?;
        Ok(())
    }

    fn save_atomic(&self, path: &PathBuf) -> anyhow::Result<()> {
        let tmp = path.with_extension("tmp");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Scoreboard {
    file: PathBuf,
}

impl Scoreboard {
    pub fn new() -> Self {
        let file = std::env::var("STATUS_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("scoreboard.json"));

        Scoreboard { file }
    }

    pub fn update_new(&self, actual_state: &OnlineStatus) {
        let now = Utc::now();
        let current_state = StreamerStalkingState::restore_or_new(&self.file.clone());

        let new_state = match actual_state {
            OnlineStatus::Offline => {
                let offline_start = current_state.last_detected_online;
                let offline_end = now;
                let offline_duration = (offline_end - offline_start).to_std().map_err(|e| {
                    warn!("Failed to calculate offline duration: {:?}", e);
                    e
                }).unwrap_or(Duration::ZERO);

                let new_highscore = if offline_duration > current_state.offline_highscore.duration {
                    Highscore {
                        duration: offline_duration,
                        start_time: offline_start,
                        end_time: offline_end,
                    }
                } else { current_state.offline_highscore };

                StreamerStalkingState {
                    last_detected_online: current_state.last_detected_online,
                    last_detected_offline: now,
                    last_update_time: now,
                    offline_highscore: new_highscore,
                    is_online: false,
                    ongoing: None,
                }
            }
            OnlineStatus::Live(stream) => {
                let rfc3339_str = "2024-03-20T14:00:00Z";
                let start_time = DateTime::parse_from_rfc3339(rfc3339_str);
                if let Err(e) = start_time {
                    warn!("Failed to parse stream start time: {:?}", e);
                    return;
                }
                let start_time = start_time.unwrap().with_timezone(&chrono::Utc);

                StreamerStalkingState {
                    last_detected_online: now,
                    last_detected_offline: current_state.last_detected_offline,
                    last_update_time: now,
                    offline_highscore: current_state.offline_highscore,
                    is_online: true,
                    ongoing: Some(OngoingStreamInfo {
                        title: stream.title.clone(),
                        game: stream.game_name.clone(),
                        started_at: start_time,
                    }),
                }
            }
        };

        if let Err(e) = new_state.save(&self.file) {
            warn!("Failed to save scoreboard: {:?}", e);
        }
    }

    pub fn get_state(&self) -> StreamerStalkingState {
        StreamerStalkingState::restore_or_new(&self.file.clone())
    }
}
