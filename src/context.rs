use crate::chat;
use crate::game_state::{BuildState, GamePhase};
use crate::match_progress::MatchProgress;
use crate::net;
use crate::role::Role;
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
    pub role: Role,
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
        self.role = if is_host { Role::Host } else { Role::Guest };

        let local = self.role.player_id() as usize;
        // TODO: 2-player assumption — derive peer index from connection identity when supporting N players
        let peer = 1 - local;

        let mut peer_name = "Opponent".to_string();
        if let Some(ref mut n) = self.net {
            n.is_host = is_host;
            peer_name = n.peer_name.clone().unwrap_or_else(|| "Opponent".to_string());
        }

        self.progress = MatchProgress::new();

        // Set names on PlayerState
        self.progress.players[local].name = player_name;
        self.progress.players[peer].name = peer_name;

        // Initialize gold with round allowance
        let allowance = self.progress.round_allowance();
        self.progress.players[local].gold = allowance;

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
            role: Role::Host,
            chat: chat::ChatState::new(),
        }
    }

}
