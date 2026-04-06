use macroquad::prelude::*;

use crate::arena::{shop_w, MatchState};
use crate::game_state::{self, BuildState, PlacedPack};
use crate::match_progress::MatchProgress;
use crate::net;
use crate::pack::all_packs;
use crate::role::Role;
use crate::settings;
use crate::team::team_color;
use crate::terrain;
use crate::unit::Unit;

pub fn draw_build_ui(
    build: &BuildState,
    progress: &MatchProgress,
    units: &[Unit],
    screen_mouse: Vec2,
    arena_camera: &Camera2D,
    mp_player_name: &str,
    mp_opponent_name: &str,
    role: Role,
) {
    crate::shop::draw_shop(build.builder.gold_remaining, screen_mouse, false, &progress.banned_kinds, game_state::BUILD_LIMIT - build.packs_bought_this_round);

    // Pack labels (drawn in screen-space so text isn't distorted by camera zoom)
    {
        let packs = all_packs();
        for placed in build.placed_packs.iter() {
            let pack = &packs[placed.pack_index];
            let half = placed.bbox_half_size_for(pack);
            let world_pos = vec2(placed.center.x - half.x + 2.0, placed.center.y - half.y - 2.0);
            let screen_pos = arena_camera.world_to_screen(world_pos);
            let label = if placed.locked {
                format!("{} (R{})", pack.name, placed.round_placed)
            } else {
                pack.name.to_string()
            };
            let label_color = if placed.locked {
                Color::new(0.5, 0.5, 0.5, 0.4)
            } else {
                Color::new(0.7, 0.7, 0.7, 0.6)
            };
            crate::ui::draw_scaled_text(&label, screen_pos.x, screen_pos.y, 14.0, label_color);
        }
        for opponent_pack in &progress.opponent(role).packs {
            let pack = &packs[opponent_pack.pack_index];
            let half = PlacedPack::bbox_half_size_rotated(pack, opponent_pack.rotated);
            let world_pos = vec2(opponent_pack.center.x - half.x + 2.0, opponent_pack.center.y - half.y - 2.0);
            let screen_pos = arena_camera.world_to_screen(world_pos);
            let label = format!("{} (R{})", pack.name, opponent_pack.round_placed);
            crate::ui::draw_scaled_text(&label, screen_pos.x, screen_pos.y, 12.0, Color::new(0.4, 0.4, 0.6, 0.4));
        }
    }

    // Tech panel (when a pack is selected)
    if let Some(sel_idx) = build.selected_pack {
        if sel_idx < build.placed_packs.len() {
            let placed = &build.placed_packs[sel_idx];
            let kind = all_packs()[placed.pack_index].kind;
            let cs = crate::tech_ui::PackCombatStats::from_units(units, &placed.unit_ids);
            crate::tech_ui::draw_tech_panel(
                kind,
                &progress.player(role).techs,
                build.builder.gold_remaining,
                screen_mouse,
                false,
                Some(&cs),
            );
        }
    }

    // Top HUD
    let army_value: u32 = {
        let packs = all_packs();
        build.placed_packs.iter().map(|p| packs[p.pack_index].cost).sum()
    };
    crate::ui::draw_hud(progress, build.builder.gold_remaining, build.timer, army_value, 0.0, mp_player_name, mp_opponent_name, role);

    // Begin Round button (screen-space)
    let btn_w = crate::ui::s(160.0);
    let btn_h = crate::ui::s(40.0);
    let btn_x = screen_width() / 2.0 - btn_w / 2.0;
    let btn_y = screen_height() - crate::ui::s(55.0);
    let btn_hovered = screen_mouse.x >= btn_x
        && screen_mouse.x <= btn_x + btn_w
        && screen_mouse.y >= btn_y
        && screen_mouse.y <= btn_y + btn_h;
    let btn_bg = if btn_hovered {
        Color::new(0.2, 0.6, 0.3, 0.9)
    } else {
        Color::new(0.15, 0.4, 0.2, 0.8)
    };
    draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_bg);
    draw_rectangle_lines(
        btn_x,
        btn_y,
        btn_w,
        btn_h,
        2.0,
        Color::new(0.3, 0.8, 0.4, 1.0),
    );
    let btn_text = "Begin Round";
    let tdims = crate::ui::measure_scaled_text(btn_text, 22);
    crate::ui::draw_scaled_text(
        btn_text,
        btn_x + btn_w / 2.0 - tdims.width / 2.0,
        btn_y + btn_h / 2.0 + 7.0,
        22.0,
        WHITE,
    );

    // Hint text (screen-space)
    crate::ui::draw_scaled_text(
        "Select -> Double-click move | Mid-click rotate | Right-click sell | G: Grid | Ctrl+Z: Undo | Scroll: Zoom",
        shop_w() + 10.0,
        screen_height() - crate::ui::s(10.0),
        13.0,
        Color::new(0.5, 0.5, 0.5, 0.7),
    );
}

pub fn draw_waiting_ui(
    progress: &MatchProgress,
    build: &BuildState,
    mp_player_name: &str,
    mp_opponent_name: &str,
    role: Role,
) {
    crate::ui::draw_hud(progress, build.builder.gold_remaining, 0.0, 0, 0.0, mp_player_name, mp_opponent_name, role);

    let dots = ".".repeat((get_time() * 2.0) as usize % 4);
    let wait_text = format!("Waiting for opponent{}", dots);
    let wdims = crate::ui::measure_scaled_text(&wait_text, 28);
    crate::ui::draw_scaled_text(
        &wait_text,
        screen_width() / 2.0 - wdims.width / 2.0,
        screen_height() / 2.0,
        28.0,
        Color::new(0.7, 0.7, 0.9, 1.0),
    );
}

pub fn draw_battle_ui(
    progress: &MatchProgress,
    units: &[Unit],
    obstacles: &[terrain::Obstacle],
    battle_timer: f32,
    round_timeout: f32,
    show_surrender_confirm: bool,
    screen_mouse: Vec2,
    world_mouse: Vec2,
    mp_player_name: &str,
    mp_opponent_name: &str,
    role: Role,
) {
    let remaining = (round_timeout - battle_timer).max(0.0);
    crate::ui::draw_hud(progress, 0, 0.0, 0, remaining, mp_player_name, mp_opponent_name, role);

    let alive_0 = units.iter().filter(|u| u.alive && u.player_id == 0).count();
    let alive_1 = units.iter().filter(|u| u.alive && u.player_id == 1).count();
    crate::ui::draw_scaled_text(
        &format!("Red: {}", alive_0),
        crate::ui::s(10.0),
        screen_height() - crate::ui::s(15.0),
        20.0,
        team_color(0),
    );
    let blue_text = format!("Blue: {}", alive_1);
    let bdims = crate::ui::measure_scaled_text(&blue_text, 20);
    crate::ui::draw_scaled_text(
        &blue_text,
        screen_width() - bdims.width - crate::ui::s(10.0),
        screen_height() - crate::ui::s(15.0),
        20.0,
        team_color(1),
    );

    // Obstacle tooltip on hover (hit test in world coords, draw in screen coords)
    if !show_surrender_confirm {
        for obs in obstacles {
            if !obs.alive { continue; }
            if obs.contains_point(world_mouse) {
                let tip_x = screen_mouse.x + crate::ui::s(15.0);
                let tip_y = (screen_mouse.y - crate::ui::s(10.0)).max(5.0);
                let tip_w = crate::ui::s(170.0);
                let tip_h = if obs.obstacle_type == terrain::ObstacleType::Cover { crate::ui::s(60.0) } else { crate::ui::s(45.0) };

                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.08, 0.08, 0.12, 0.95));
                draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

                let type_name = match obs.obstacle_type {
                    terrain::ObstacleType::Wall => "Wall (Indestructible)",
                    terrain::ObstacleType::Cover => "Cover (Destructible)",
                };
                crate::ui::draw_scaled_text(type_name, tip_x + crate::ui::s(6.0), tip_y + crate::ui::s(16.0), 14.0, WHITE);

                let mut ty = tip_y + crate::ui::s(32.0);
                if obs.obstacle_type == terrain::ObstacleType::Cover {
                    crate::ui::draw_scaled_text(&format!("HP: {:.0}/{:.0}", obs.hp, obs.max_hp), tip_x + crate::ui::s(6.0), ty, 12.0, LIGHTGRAY);
                    ty += crate::ui::s(14.0);
                }
                let team_name = match obs.player_id { 0 => mp_player_name, 1 => mp_opponent_name, _ => "Neutral" };
                crate::ui::draw_scaled_text(&format!("Owner: {}", team_name), tip_x + crate::ui::s(6.0), ty, 12.0, LIGHTGRAY);
                break;
            }
        }
    }

    // Surrender confirmation overlay
    if show_surrender_confirm {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
        let title = "Surrender?";
        let tdims = crate::ui::measure_scaled_text(title, 36);
        crate::ui::draw_scaled_text(title, screen_width() / 2.0 - tdims.width / 2.0, screen_height() / 2.0 - crate::ui::s(20.0), 36.0, WHITE);

        let btn_w: f32 = crate::ui::s(120.0);
        let btn_h: f32 = crate::ui::s(40.0);
        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        // Yes button
        let yes_x = cx - btn_w - crate::ui::s(10.0);
        let yes_y = cy + crate::ui::s(10.0);
        let yes_hover = screen_mouse.x >= yes_x && screen_mouse.x <= yes_x + btn_w && screen_mouse.y >= yes_y && screen_mouse.y <= yes_y + btn_h;
        let yes_color = if yes_hover { Color::new(0.8, 0.2, 0.2, 0.9) } else { Color::new(0.6, 0.15, 0.15, 0.8) };
        draw_rectangle(yes_x, yes_y, btn_w, btn_h, yes_color);
        draw_rectangle_lines(yes_x, yes_y, btn_w, btn_h, 1.0, WHITE);
        let yt = "Yes";
        let ydims = crate::ui::measure_scaled_text(yt, 20);
        crate::ui::draw_scaled_text(yt, yes_x + btn_w / 2.0 - ydims.width / 2.0, yes_y + btn_h / 2.0 + 6.0, 20.0, WHITE);

        // Cancel button
        let no_x = cx + crate::ui::s(10.0);
        let no_y = cy + crate::ui::s(10.0);
        let no_hover = screen_mouse.x >= no_x && screen_mouse.x <= no_x + btn_w && screen_mouse.y >= no_y && screen_mouse.y <= no_y + btn_h;
        let no_color = if no_hover { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
        draw_rectangle(no_x, no_y, btn_w, btn_h, no_color);
        draw_rectangle_lines(no_x, no_y, btn_w, btn_h, 1.0, WHITE);
        let nt = "Cancel";
        let ndims = crate::ui::measure_scaled_text(nt, 20);
        crate::ui::draw_scaled_text(nt, no_x + btn_w / 2.0 - ndims.width / 2.0, no_y + btn_h / 2.0 + 6.0, 20.0, WHITE);
    }
}

pub fn draw_round_result_ui(
    progress: &MatchProgress,
    match_state: &MatchState,
    lp_damage: i32,
    loser_team: Option<u8>,
    game_settings: &settings::GameSettings,
    net: &Option<net::NetState>,
    mp_player_name: &str,
    mp_opponent_name: &str,
    role: Role,
) {
    crate::ui::draw_hud(progress, 0, 0.0, 0, 0.0, mp_player_name, mp_opponent_name, role);

    let text = match match_state {
        MatchState::Winner(tid) => {
            let (winner_name, color_idx) = if *tid == 0 {
                (mp_player_name, game_settings.player_color_index)
            } else {
                let opp_idx = net.as_ref().and_then(|n| n.opponent_color).unwrap_or(1);
                (mp_opponent_name, opp_idx)
            };
            let color_name = settings::TEAM_COLOR_OPTIONS
                .get(color_idx as usize)
                .map(|(name, _)| *name)
                .unwrap_or("???");
            format!("{} ({}) wins round {}!", winner_name, color_name, progress.round)
        }
        MatchState::Draw => format!("Round {} - Draw!", progress.round),
        MatchState::InProgress => unreachable!(),
    };

    let dims = crate::ui::measure_scaled_text(&text, 36);
    crate::ui::draw_scaled_text(
        &text,
        screen_width() / 2.0 - dims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(30.0),
        36.0,
        WHITE,
    );

    if let Some(loser) = loser_team {
        let loser_name = if loser == 0 { mp_player_name } else { mp_opponent_name };
        let dmg_text = format!("{} loses {} LP", loser_name, lp_damage);
        let ddims = crate::ui::measure_scaled_text(&dmg_text, 22);
        crate::ui::draw_scaled_text(
            &dmg_text,
            screen_width() / 2.0 - ddims.width / 2.0,
            screen_height() / 2.0 + crate::ui::s(5.0),
            22.0,
            Color::new(1.0, 0.4, 0.3, 1.0),
        );
    }

    let next_text = if progress.is_game_over() {
        "Press Space to see results"
    } else {
        "Press Space for next round"
    };
    let ndims = crate::ui::measure_scaled_text(next_text, 18);
    crate::ui::draw_scaled_text(
        next_text,
        screen_width() / 2.0 - ndims.width / 2.0,
        screen_height() / 2.0 + crate::ui::s(35.0),
        18.0,
        LIGHTGRAY,
    );
}

pub fn draw_game_over_ui(
    winner: u8,
    progress: &MatchProgress,
    units: &[Unit],
    game_settings: &settings::GameSettings,
    net: &Option<net::NetState>,
    screen_mouse: Vec2,
    mp_player_name: &str,
    mp_opponent_name: &str,
    role: Role,
) {
    let local_pid = role.player_id();
    let (headline, winner_color_idx) = if winner == local_pid {
        ("YOU WIN!".to_string(), game_settings.player_color_index)
    } else {
        ("YOU LOSE!".to_string(), net.as_ref().and_then(|n| n.opponent_color).unwrap_or(1))
    };
    let winner_name = if winner == local_pid { mp_player_name } else { mp_opponent_name };
    let color_name = settings::TEAM_COLOR_OPTIONS
        .get(winner_color_idx as usize)
        .map(|(name, _)| *name)
        .unwrap_or("???");
    let subtitle = format!("{} ({}) wins!", winner_name, color_name);
    let headline_color = if winner == local_pid {
        Color::new(0.2, 1.0, 0.3, 1.0)
    } else {
        Color::new(1.0, 0.3, 0.2, 1.0)
    };
    let dims = crate::ui::measure_scaled_text(&headline, 48);
    crate::ui::draw_scaled_text(
        &headline,
        screen_width() / 2.0 - dims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(40.0),
        48.0,
        headline_color,
    );
    let sub_dims = crate::ui::measure_scaled_text(&subtitle, 22);
    let (_, (cr, cg, cb)) = settings::TEAM_COLOR_OPTIONS
        .get(winner_color_idx as usize)
        .copied()
        .unwrap_or(("White", (1.0, 1.0, 1.0)));
    crate::ui::draw_scaled_text(
        &subtitle,
        screen_width() / 2.0 - sub_dims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(10.0),
        22.0,
        Color::new(cr, cg, cb, 1.0),
    );

    // Stats panel
    let panel_w = crate::ui::s(320.0);
    let panel_h = crate::ui::s(140.0);
    let panel_x = screen_width() / 2.0 - panel_w / 2.0;
    let panel_y = screen_height() / 2.0 + 10.0;
    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.08, 0.08, 0.12, 0.9));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0, Color::new(0.4, 0.5, 0.6, 0.7));

    let mut sy = panel_y + crate::ui::s(18.0);
    let sx = panel_x + crate::ui::s(12.0);

    let round_text = format!("Rounds Played: {}", progress.round);
    crate::ui::draw_scaled_text(&round_text, sx, sy, 15.0, LIGHTGRAY);
    sy += crate::ui::s(18.0);

    // MVP
    let mvp = units.iter()
        .filter(|u| u.player_id == local_pid)
        .max_by(|a, b| a.damage_dealt_total.partial_cmp(&b.damage_dealt_total).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(mvp_unit) = mvp {
        let mvp_text = format!("MVP: {:?} - {:.0} dmg, {} kills", mvp_unit.kind, mvp_unit.damage_dealt_total, mvp_unit.kills_total);
        crate::ui::draw_scaled_text(&mvp_text, sx, sy, 15.0, Color::new(1.0, 0.85, 0.2, 1.0));
    }
    sy += crate::ui::s(18.0);

    let total_dmg: f32 = units.iter()
        .filter(|u| u.player_id == local_pid)
        .map(|u| u.damage_dealt_total)
        .sum();
    crate::ui::draw_scaled_text(&format!("Total Damage: {:.0}", total_dmg), sx, sy, 15.0, LIGHTGRAY);
    sy += crate::ui::s(18.0);

    let surviving = units.iter().filter(|u| u.player_id == local_pid && u.alive).count();
    let total_units = units.iter().filter(|u| u.player_id == local_pid).count();
    crate::ui::draw_scaled_text(&format!("Surviving: {} / {}", surviving, total_units), sx, sy, 15.0, LIGHTGRAY);
    sy += crate::ui::s(18.0);

    crate::ui::draw_scaled_text(&format!("LP: {} {} vs {} {}", mp_player_name, progress.player(role).lp, mp_opponent_name, progress.opponent(role).lp), sx, sy, 15.0, LIGHTGRAY);

    let below_panel = panel_y + panel_h + crate::ui::s(8.0);
    crate::ui::draw_scaled_text(
        "Press R to return to lobby",
        screen_width() / 2.0 - crate::ui::s(100.0),
        below_panel,
        16.0,
        DARKGRAY,
    );

    // Rematch button
    let rmatch_w = crate::ui::s(160.0);
    let rmatch_h = crate::ui::s(40.0);
    let rmatch_x = screen_width() / 2.0 - rmatch_w / 2.0;
    let rmatch_y = below_panel + crate::ui::s(15.0);
    let rmatch_hover = screen_mouse.x >= rmatch_x && screen_mouse.x <= rmatch_x + rmatch_w && screen_mouse.y >= rmatch_y && screen_mouse.y <= rmatch_y + rmatch_h;
    let rmatch_bg = if rmatch_hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
    draw_rectangle(rmatch_x, rmatch_y, rmatch_w, rmatch_h, rmatch_bg);
    draw_rectangle_lines(rmatch_x, rmatch_y, rmatch_w, rmatch_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
    let rt = "Rematch";
    let rdims2 = crate::ui::measure_scaled_text(rt, 22);
    crate::ui::draw_scaled_text(rt, rmatch_x + rmatch_w / 2.0 - rdims2.width / 2.0, rmatch_y + rmatch_h / 2.0 + 7.0, 22.0, WHITE);
}

pub fn draw_disconnect_overlay() {
    // Semi-transparent dark overlay
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.7));
    let disc_text = "Opponent Disconnected";
    let ddims = crate::ui::measure_scaled_text(disc_text, 36);
    crate::ui::draw_scaled_text(
        disc_text,
        screen_width() / 2.0 - ddims.width / 2.0,
        screen_height() / 2.0 - crate::ui::s(10.0),
        36.0,
        Color::new(1.0, 0.3, 0.2, 1.0),
    );
    let hint = "Press R to return to lobby";
    let hdims = crate::ui::measure_scaled_text(hint, 18);
    crate::ui::draw_scaled_text(
        hint,
        screen_width() / 2.0 - hdims.width / 2.0,
        screen_height() / 2.0 + crate::ui::s(20.0),
        18.0,
        LIGHTGRAY,
    );
}
