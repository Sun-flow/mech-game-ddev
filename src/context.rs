use crate::chat;
use crate::game_state::{BuildState, GamePhase};
use crate::match_progress::MatchProgress;
use crate::net;
use crate::settings;
use crate::terrain;
use crate::unit::Unit;

pub struct GameContext {
    pub progress: MatchProgress,
    pub phase: GamePhase,
    pub build: BuildState,
    pub units: Vec<Unit>,
    pub net: Option<net::NetState>,
    pub obstacles: Vec<terrain::Obstacle>,
    pub nav_grid: Option<terrain::NavGrid>,
    pub game_settings: settings::GameSettings,
    pub show_grid: bool,
    pub mp_player_name: String,
    pub mp_opponent_name: String,
    pub chat: chat::ChatState,
}

impl GameContext {
    pub fn new(is_host: bool) -> Self {
        let progress = MatchProgress::new(is_host);
        let build = BuildState::new(progress.round_gold(), is_host);
        Self {
            progress,
            phase: GamePhase::Lobby,
            build,
            units: Vec::new(),
            net: None,
            obstacles: Vec::new(),
            nav_grid: None,
            game_settings: settings::GameSettings::default(),
            show_grid: false,
            mp_player_name: String::from("Player"),
            mp_opponent_name: String::from("Opponent"),
            chat: chat::ChatState::new(),
        }
    }
}
