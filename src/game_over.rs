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
        ctx.progress = MatchProgress::new(true);
        ctx.phase = GamePhase::Lobby;
        ctx.build = BuildState::new(ctx.progress.round_gold(), true);
        ctx.units.clear();
        battle.projectiles.clear();
        ctx.net = None;
        lobby.reset();
    }

    let rmatch_w = crate::ui::s(160.0);
    let rmatch_h = crate::ui::s(40.0);
    let rmatch_x = screen_width() / 2.0 - rmatch_w / 2.0;
    let rmatch_panel_y = screen_height() / 2.0 + 10.0;
    let rmatch_panel_h = crate::ui::s(140.0);
    let rmatch_y = rmatch_panel_y + rmatch_panel_h + crate::ui::s(8.0) + crate::ui::s(15.0);
    if left_click && screen_mouse.x >= rmatch_x && screen_mouse.x <= rmatch_x + rmatch_w
        && screen_mouse.y >= rmatch_y && screen_mouse.y <= rmatch_y + rmatch_h
    {
        let is_host = ctx.net.as_ref().is_none_or(|n| n.is_host);
        ctx.progress = MatchProgress::new(is_host);
        ctx.build = BuildState::new(ctx.progress.round_gold(), is_host);
        ctx.units.clear();
        ctx.obstacles.clear();
        ctx.nav_grid = None;
        ctx.chat = chat::ChatState::new();
        battle.reset();
        ctx.phase = if ctx.game_settings.draft_ban_enabled {
            GamePhase::DraftBan { bans: Vec::new(), confirmed: false, opponent_bans: None }
        } else {
            GamePhase::Build
        };
    }
}
