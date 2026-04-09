use macroquad::prelude::*;
use crate::net;
use crate::unit::UnitKind;

pub const ALL_KINDS: [UnitKind; 13] = [
    UnitKind::Striker, UnitKind::Sentinel, UnitKind::Ranger, UnitKind::Scout,
    UnitKind::Bruiser, UnitKind::Artillery, UnitKind::Chaff, UnitKind::Sniper,
    UnitKind::Skirmisher, UnitKind::Dragoon, UnitKind::Berserker,
    UnitKind::Shield, UnitKind::Interceptor,
];

pub enum DraftBanResult {
    Waiting,
    Done(Vec<UnitKind>),
}

pub fn update_and_draw(
    bans: &mut Vec<UnitKind>,
    confirmed: &mut bool,
    peer_bans: &mut Option<Vec<UnitKind>>,
    net: &mut Option<crate::net::NetState>,
    screen_mouse: Vec2,
    left_click: bool,
) -> DraftBanResult {
    let all_kinds = ALL_KINDS;

    // Draw background
    clear_background(Color::new(0.08, 0.08, 0.12, 1.0));

    // Title
    crate::ui::draw_centered_text("Ban Phase — Select up to 2 unit types to ban", screen_width() / 2.0, crate::ui::s(50.0), 24.0, WHITE);

    // Draw unit cards in a grid (4 cols)
    let cols = 4;
    let card_w = crate::ui::s(160.0);
    let card_h = crate::ui::s(50.0);
    let gap = crate::ui::s(12.0);
    let grid_w = cols as f32 * card_w + (cols - 1) as f32 * gap;
    let start_x = screen_width() / 2.0 - grid_w / 2.0;
    let start_y = crate::ui::s(90.0);

    for (i, kind) in all_kinds.iter().enumerate() {
        let col = (i % cols) as f32;
        let row = (i / cols) as f32;
        let x = start_x + col * (card_w + gap);
        let y = start_y + row * (card_h + gap);

        let is_banned = bans.contains(kind);
        let is_hovered = crate::ui::point_in_rect(screen_mouse, x, y, card_w, card_h);

        let bg = if is_banned {
            Color::new(0.6, 0.15, 0.15, 0.9)
        } else if is_hovered {
            Color::new(0.2, 0.25, 0.35, 0.9)
        } else {
            Color::new(0.12, 0.12, 0.18, 0.9)
        };

        draw_rectangle(x, y, card_w, card_h, bg);
        draw_rectangle_lines(x, y, card_w, card_h, 1.0, if is_banned { RED } else { GRAY });

        let name = format!("{:?}", kind);
        let stats = kind.stats();
        let info = format!("{} HP:{:.0} DMG:{:.0}", name, stats.max_hp, stats.damage);
        crate::ui::draw_scaled_text(&info, x + crate::ui::s(8.0), y + crate::ui::s(20.0), 14.0, if is_banned { Color::new(1.0, 0.5, 0.5, 1.0) } else { WHITE });

        if is_banned {
            crate::ui::draw_centered_text("BANNED", x + card_w / 2.0, y + crate::ui::s(40.0), 16.0, RED);
        } else {
            let detail = format!("RNG:{:.0} SPD:{:.0} AS:{:.1}", stats.attack_range, stats.move_speed, stats.attack_speed);
            crate::ui::draw_scaled_text(&detail, x + crate::ui::s(8.0), y + crate::ui::s(38.0), 12.0, LIGHTGRAY);
        }

        // Click to toggle ban
        if left_click && is_hovered {
            if is_banned {
                bans.retain(|k| k != kind);
            } else if bans.len() < 2 {
                bans.push(*kind);
            }
        }
    }

    // Confirm button
    let btn_w = crate::ui::s(200.0);
    let btn_h = crate::ui::s(45.0);
    let btn_x = crate::ui::center_x(btn_w);
    let btn_y = start_y + 4.0 * (card_h + gap) + crate::ui::s(20.0);
    let btn_hover = crate::ui::point_in_rect(screen_mouse, btn_x, btn_y, btn_w, btn_h);
    let btn_color = if btn_hover { Color::new(0.2, 0.6, 0.3, 0.9) } else { Color::new(0.15, 0.45, 0.2, 0.8) };
    draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
    draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.0, WHITE);
    let confirm_text = format!("Confirm Bans ({}/ 2)", bans.len());
    crate::ui::draw_centered_text(&confirm_text, btn_x + btn_w / 2.0, btn_y + btn_h / 2.0 + 6.0, 20.0, WHITE);

    // Poll network for peer bans
    if let Some(ref mut n) = net {
        n.poll();
        if let Some(ob) = n.peer_bans.take() {
            let opp: Vec<UnitKind> = ob.iter().filter_map(|&idx| {
                all_kinds.get(idx as usize).copied()
            }).collect();
            *peer_bans = Some(opp);
        }
    }

    // Confirm button click: lock in our bans and send to opponent
    if left_click && btn_hover && !*confirmed {
        *confirmed = true;
        if let Some(ref mut n) = net {
            let ban_indices: Vec<u8> = bans.iter().map(|k| {
                all_kinds.iter().position(|ak| ak == k).unwrap_or(0) as u8
            }).collect();
            n.send(net::NetMessage::BanSelection(ban_indices));
        }
    }

    // Show waiting indicator
    if *confirmed && net.is_some() && peer_bans.is_none() {
        let wait_y = btn_y + btn_h + crate::ui::s(15.0);
        let dots = ".".repeat((get_time() * 2.0) as usize % 4);
        let wait_text = format!("Waiting for opponent bans{}", dots);
        crate::ui::draw_centered_text(&wait_text, screen_width() / 2.0, wait_y, 16.0, LIGHTGRAY);
    }

    // Transition when ready
    let ready = *confirmed && (net.is_none() || peer_bans.is_some());
    if ready {
        let mut all_bans = bans.clone();
        if let Some(ref ob) = peer_bans {
            all_bans.extend(ob.iter());
        }
        return DraftBanResult::Done(all_bans);
    }

    DraftBanResult::Waiting
}
