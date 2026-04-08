use crate::arena;
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
    pub local_player_id: u8,
    pub chat: chat::ChatState,
}

impl GameContext {
    /// Transition from lobby to the first gameplay phase.
    pub fn start_game(
        &mut self,
        net: Option<net::NetState>,
        is_host: bool,
        player_name: String,
        draft_ban_enabled: bool,
    ) {
        self.net = net;
        self.local_player_id = if is_host { 0 } else { 1 };

        let mut peer_name = "Opponent".to_string();
        if let Some(ref mut n) = self.net {
            n.is_host = is_host;
            // peer_name is Option<(u8, String)> after Task 2 changes net.rs
            // For now use the current type; Task 2 will update net.rs
            peer_name = n.peer_name.clone().unwrap_or_else(|| "Opponent".to_string());
        }

        self.progress = MatchProgress::new();

        // Set names using canonical player_id
        self.progress.players[self.local_player_id as usize].name = player_name;
        // Set peer name on all other players
        for (i, p) in self.progress.players.iter_mut().enumerate() {
            if i != self.local_player_id as usize {
                p.name = peer_name.clone();
            }
        }

        // Initialize gold with round allowance
        let allowance = self.progress.round_allowance();
        self.progress.players[self.local_player_id as usize].gold = allowance;

        self.build = BuildState::new(allowance, is_host);
        if draft_ban_enabled {
            self.phase = GamePhase::DraftBan {
                bans: Vec::new(),
                confirmed: false,
                peer_bans: None,
            };
        } else {
            self.phase = GamePhase::Build;
        }
    }

    pub fn new() -> Self {
        let progress = MatchProgress::new();
        let allowance = progress.round_allowance();
        let build = BuildState::new(allowance, true);
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
            local_player_id: 0,
            chat: chat::ChatState::new(),
        }
    }

}
