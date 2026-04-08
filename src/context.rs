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
    pub local_player_id: u16,
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
        local_player_id: u16,
        peer_player_id: Option<u16>,
    ) {
        self.net = net;
        self.local_player_id = local_player_id;

        // Build player ID list
        let mut player_ids = vec![local_player_id];
        if let Some(ppid) = peer_player_id {
            player_ids.push(ppid);
        }

        self.progress = MatchProgress::new(&player_ids);

        // Set names
        self.progress.player_mut(local_player_id).name = player_name;
        if let Some(ref n) = self.net {
            if let Some((pid, name)) = n.peer_name.clone() {
                self.progress.player_mut(pid).name = name;
            }
        }

        // Assign deploy zones
        let left = (0.0f32, crate::arena::HALF_W);
        let right = (crate::arena::HALF_W, crate::arena::ARENA_W);
        if is_host {
            self.progress.player_mut(local_player_id).deploy_zone = left;
            if let Some(ppid) = peer_player_id {
                self.progress.player_mut(ppid).deploy_zone = right;
            }
        } else {
            self.progress.player_mut(local_player_id).deploy_zone = right;
            if let Some(ppid) = peer_player_id {
                self.progress.player_mut(ppid).deploy_zone = left;
            }
        }

        // Set colors
        self.progress.player_mut(local_player_id).color_index = self.game_settings.player_color_index;
        if let Some(ref n) = self.net {
            if let Some((pid, color_idx)) = n.peer_color {
                self.progress.player_mut(pid).color_index = color_idx;
            }
        }

        // Initialize gold
        let allowance = self.progress.round_allowance();
        self.progress.player_mut(local_player_id).gold = allowance;

        let next_id = self.progress.player(local_player_id).next_id;
        self.build = BuildState::new(allowance, next_id);

        if let Some(ref mut n) = self.net {
            n.is_host = is_host;
        }

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
        let progress = MatchProgress::new(&[0]);
        let allowance = progress.round_allowance();
        let build = BuildState::new(allowance, 1);
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
