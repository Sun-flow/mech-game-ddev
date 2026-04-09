use macroquad::prelude::*;

use crate::battle_phase::BattleState;
use crate::chat;
use crate::context::GameContext;
use crate::game_state::{BuildState, GamePhase};
use crate::lobby;
use crate::match_progress::MatchProgress;

pub fn update(
    ctx: &mut GameContext,
    battle: &mut BattleState,
    lobby: &mut lobby::LobbyState,
    screen_mouse: Vec2,
    left_click: bool,
) {
    if is_key_pressed(KeyCode::R) {
        ctx.progress = MatchProgress::new(&[0]);
        ctx.phase = GamePhase::Lobby;
        ctx.build = BuildState::new(ctx.progress.round_allowance(), 1);
        ctx.units.clear();
        battle.projectiles.clear();
        ctx.net = None;
        lobby.reset();
    }

    let rmatch_w = crate::ui::s(160.0);
    let rmatch_h = crate::ui::s(40.0);
    let rmatch_x = crate::ui::center_x(rmatch_w);
    let rmatch_panel_y = screen_height() / 2.0 + 10.0;
    let rmatch_panel_h = crate::ui::s(140.0);
    let rmatch_y = rmatch_panel_y + rmatch_panel_h + crate::ui::s(8.0) + crate::ui::s(15.0);
    if left_click && crate::ui::point_in_rect(screen_mouse, rmatch_x, rmatch_y, rmatch_w, rmatch_h) {
        // Preserve player IDs, deploy zones, colors, and names across rematch
        let player_info: Vec<(u16, (f32, f32), u8, String)> = ctx.progress.players.iter()
            .map(|p| (p.player_id, p.deploy_zone, p.color_index, p.name.clone()))
            .collect();
        let player_ids: Vec<u16> = player_info.iter().map(|(pid, _, _, _)| *pid).collect();
        ctx.progress = MatchProgress::new(&player_ids);
        for (pid, zone, color, name) in &player_info {
            let p = ctx.progress.player_mut(*pid);
            p.deploy_zone = *zone;
            p.color_index = *color;
            p.name = name.clone();
        }
        let allowance = ctx.progress.round_allowance();
        ctx.build = BuildState::new(allowance, ctx.progress.player(ctx.local_player_id).next_id);
        ctx.units.clear();
        ctx.obstacles.clear();
        ctx.nav_grid = None;
        ctx.chat = chat::ChatState::new();
        battle.reset();
        ctx.phase = if ctx.game_settings.draft_ban_enabled {
            GamePhase::DraftBan { bans: Vec::new(), confirmed: false, peer_bans: None }
        } else {
            GamePhase::Build
        };
    }
}
