use macroquad::prelude::*;

use crate::arena::{ARENA_H, ARENA_W};
use crate::net::NetState;

#[derive(PartialEq)]
pub enum LobbyMode {
    Menu,
    Settings,
    MatchSettings { next_action: MatchSettingsNext },
    EnteringCode,
    WaitingForPeer,
    Connected,
}

#[derive(Clone, PartialEq)]
pub enum MatchSettingsNext {
    CreateRoom,
    VsAi,
}

/// Result of lobby update - what should the game do next?
pub enum LobbyResult {
    /// Still in lobby, keep waiting
    Waiting,
    /// Start multiplayer game (net state is in lobby.net)
    StartMultiplayer,
    /// Start single-player vs AI
    StartVsAi,
}

pub struct LobbyState {
    pub mode: LobbyMode,
    pub room_code: String,
    pub input_code: String,
    pub status: String,
    pub net: Option<NetState>,
    pub player_name: String,
    pub name_editing: bool,
}

impl LobbyState {
    pub fn new() -> Self {
        Self {
            mode: LobbyMode::Menu,
            room_code: String::new(),
            input_code: String::new(),
            status: String::new(),
            net: None,
            player_name: "Player".to_string(),
            name_editing: false,
        }
    }

    pub fn reset(&mut self) {
        self.mode = LobbyMode::Menu;
        self.room_code.clear();
        self.input_code.clear();
        self.status.clear();
        self.net = None;
    }

    fn generate_room_code() -> String {
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        (0..4)
            .map(|_| {
                let idx = macroquad::rand::gen_range(0, chars.len());
                chars[idx] as char
            })
            .collect()
    }

    pub fn update(&mut self, game_settings: &mut crate::settings::GameSettings) -> LobbyResult {
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let left_click = is_mouse_button_pressed(MouseButton::Left);

        let btn_w = 240.0;
        let btn_h = 45.0;
        let btn_x = ARENA_W / 2.0 - btn_w / 2.0;

        // Clone next_action if in MatchSettings to avoid borrow issues
        let match_settings_next = if let LobbyMode::MatchSettings { ref next_action } = self.mode {
            Some(next_action.clone())
        } else {
            None
        };

        match self.mode {
            LobbyMode::Menu => {
                // Name field editing
                let name_y = ARENA_H / 2.0 - 80.0;
                let name_w = 200.0;
                let name_x = ARENA_W / 2.0 - name_w / 2.0;
                let name_h = 30.0;
                if left_click && mouse.x >= name_x && mouse.x <= name_x + name_w
                    && mouse.y >= name_y && mouse.y <= name_y + name_h {
                    self.name_editing = true;
                } else if left_click {
                    self.name_editing = false;
                }
                if self.name_editing {
                    while let Some(ch) = get_char_pressed() {
                        if ch == '\u{8}' { self.player_name.pop(); }
                        else if self.player_name.len() < 16 && (ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ' ') {
                            self.player_name.push(ch);
                        }
                    }
                }

                let create_y = ARENA_H / 2.0 - 25.0;

                // "Create Room" → go to Match Settings
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= create_y && mouse.y <= create_y + btn_h
                {
                    self.mode = LobbyMode::MatchSettings { next_action: MatchSettingsNext::CreateRoom };
                }

                // "Join Room" button
                let join_y = create_y + 55.0;
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= join_y && mouse.y <= join_y + btn_h
                {
                    self.input_code.clear();
                    self.mode = LobbyMode::EnteringCode;
                }

                // "Play vs AI" → go to Match Settings
                let ai_y = join_y + 55.0;
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= ai_y && mouse.y <= ai_y + btn_h
                {
                    self.mode = LobbyMode::MatchSettings { next_action: MatchSettingsNext::VsAi };
                }

                // "Settings" button (placeholder)
                let settings_y = ai_y + 55.0;
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= settings_y && mouse.y <= settings_y + btn_h
                {
                    self.mode = LobbyMode::Settings;
                }
            }

            LobbyMode::Settings => {
                if is_key_pressed(KeyCode::Escape) {
                    self.mode = LobbyMode::Menu;
                }
            }

            LobbyMode::MatchSettings { .. } => {
                // Drawing and click handling done in draw() method
                if is_key_pressed(KeyCode::Escape) {
                    self.mode = LobbyMode::Menu;
                }
            }

            LobbyMode::EnteringCode => {
                while let Some(ch) = get_char_pressed() {
                    if ch.is_alphanumeric() && self.input_code.len() < 4 {
                        self.input_code.push(ch.to_ascii_uppercase());
                    }
                }
                if is_key_pressed(KeyCode::Backspace) && !self.input_code.is_empty() {
                    self.input_code.pop();
                }
                if is_key_pressed(KeyCode::Escape) {
                    self.mode = LobbyMode::Menu;
                }

                if self.input_code.len() == 4 {
                    let connect_x = ARENA_W / 2.0 - 80.0;
                    let connect_y = ARENA_H / 2.0 + 50.0;
                    let cbw = 160.0;
                    let cbh = 40.0;

                    let should_connect = (left_click
                        && mouse.x >= connect_x && mouse.x <= connect_x + cbw
                        && mouse.y >= connect_y && mouse.y <= connect_y + cbh)
                        || is_key_pressed(KeyCode::Enter);

                    if should_connect {
                        self.room_code = self.input_code.clone();
                        self.net = Some(NetState::new(&self.room_code));
                        self.status = format!("Joining room {}...", self.room_code);
                        self.mode = LobbyMode::WaitingForPeer;
                    }
                }
            }

            LobbyMode::WaitingForPeer => {
                if let Some(ref mut net) = self.net {
                    net.poll();

                    if net.disconnected {
                        self.status = "Connection failed. Press Escape to retry.".to_string();
                        if is_key_pressed(KeyCode::Escape) {
                            self.reset();
                        }
                        return LobbyResult::Waiting;
                    }

                    if net.is_peer_connected() {
                        // Only the host sends match settings to the guest
                        if net.is_host {
                            net.send(crate::net::NetMessage::SettingsSync(game_settings.clone()));
                        }
                        net.send(crate::net::NetMessage::ReadyToStart);
                        self.status = "Peer connected! Waiting for ready...".to_string();
                        self.mode = LobbyMode::Connected;
                    }
                }

                if is_key_pressed(KeyCode::Escape) {
                    self.reset();
                }
            }

            LobbyMode::Connected => {
                if let Some(ref mut net) = self.net {
                    net.poll();

                    if net.disconnected {
                        self.status = "Opponent disconnected.".to_string();
                        return LobbyResult::Waiting;
                    }

                    // Apply host's match settings (preserve local player color)
                    if let Some(settings) = net.received_settings.take() {
                        let my_color = game_settings.player_color_index;
                        *game_settings = settings;
                        game_settings.player_color_index = my_color;
                    }

                    if net.peer_ready {
                        return LobbyResult::StartMultiplayer;
                    }
                }
            }
        }

        LobbyResult::Waiting
    }

    pub fn draw(&mut self, game_settings: &mut crate::settings::GameSettings) -> LobbyResult {
        clear_background(Color::new(0.08, 0.08, 0.1, 1.0));

        let title = "RTS Unit Arena";
        let tdims = measure_text(title, None, 40, 1.0);
        draw_text(title, ARENA_W / 2.0 - tdims.width / 2.0, ARENA_H / 2.0 - 140.0, 40.0, WHITE);

        let subtitle = "Multiplayer";
        let sdims = measure_text(subtitle, None, 24, 1.0);
        draw_text(subtitle, ARENA_W / 2.0 - sdims.width / 2.0, ARENA_H / 2.0 - 105.0, 24.0, Color::new(0.5, 0.7, 1.0, 1.0));

        let mouse = vec2(mouse_position().0, mouse_position().1);
        let btn_w = 240.0;
        let btn_h = 45.0;
        let btn_x = ARENA_W / 2.0 - btn_w / 2.0;

        match self.mode {
            LobbyMode::Menu => {
                // Player name field
                let name_y = ARENA_H / 2.0 - 80.0;
                let name_w = 200.0;
                let name_x = ARENA_W / 2.0 - name_w / 2.0;
                let name_h = 30.0;
                draw_text("Name:", name_x - 50.0, name_y + 20.0, 16.0, LIGHTGRAY);
                let name_bg = if self.name_editing { Color::new(0.15, 0.15, 0.22, 0.95) } else { Color::new(0.1, 0.1, 0.15, 0.8) };
                draw_rectangle(name_x, name_y, name_w, name_h, name_bg);
                let border_color = if self.name_editing { Color::new(0.4, 0.7, 1.0, 1.0) } else { Color::new(0.3, 0.3, 0.4, 0.8) };
                draw_rectangle_lines(name_x, name_y, name_w, name_h, 1.0, border_color);
                let cursor = if self.name_editing && (get_time() * 2.0) as u32 % 2 == 0 { "|" } else { "" };
                draw_text(&format!("{}{}", self.player_name, cursor), name_x + 6.0, name_y + 20.0, 16.0, WHITE);

                let create_y = ARENA_H / 2.0 - 25.0;

                // Create Room
                let hover = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= create_y && mouse.y <= create_y + btn_h;
                let bg = if hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
                draw_rectangle(btn_x, create_y, btn_w, btn_h, bg);
                draw_rectangle_lines(btn_x, create_y, btn_w, btn_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                let t = "Create Room";
                let d = measure_text(t, None, 22, 1.0);
                draw_text(t, btn_x + btn_w / 2.0 - d.width / 2.0, create_y + btn_h / 2.0 + 7.0, 22.0, WHITE);

                // Join Room
                let join_y = create_y + 55.0;
                let hover2 = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= join_y && mouse.y <= join_y + btn_h;
                let bg2 = if hover2 { Color::new(0.2, 0.3, 0.5, 0.9) } else { Color::new(0.15, 0.2, 0.35, 0.8) };
                draw_rectangle(btn_x, join_y, btn_w, btn_h, bg2);
                draw_rectangle_lines(btn_x, join_y, btn_w, btn_h, 2.0, Color::new(0.3, 0.5, 0.9, 1.0));
                let t2 = "Join Room";
                let d2 = measure_text(t2, None, 22, 1.0);
                draw_text(t2, btn_x + btn_w / 2.0 - d2.width / 2.0, join_y + btn_h / 2.0 + 7.0, 22.0, WHITE);

                // Play vs AI
                let ai_y = join_y + 55.0;
                let hover3 = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= ai_y && mouse.y <= ai_y + btn_h;
                let bg3 = if hover3 { Color::new(0.4, 0.3, 0.2, 0.9) } else { Color::new(0.3, 0.2, 0.15, 0.8) };
                draw_rectangle(btn_x, ai_y, btn_w, btn_h, bg3);
                draw_rectangle_lines(btn_x, ai_y, btn_w, btn_h, 2.0, Color::new(0.9, 0.6, 0.3, 1.0));
                let t3 = "Play vs AI";
                let d3 = measure_text(t3, None, 22, 1.0);
                draw_text(t3, btn_x + btn_w / 2.0 - d3.width / 2.0, ai_y + btn_h / 2.0 + 7.0, 22.0, WHITE);

                // Settings
                let settings_y = ai_y + 55.0;
                let hover4 = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= settings_y && mouse.y <= settings_y + btn_h;
                let bg4 = if hover4 { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
                draw_rectangle(btn_x, settings_y, btn_w, btn_h, bg4);
                draw_rectangle_lines(btn_x, settings_y, btn_w, btn_h, 2.0, Color::new(0.6, 0.6, 0.7, 1.0));
                let t4 = "Settings";
                let d4 = measure_text(t4, None, 22, 1.0);
                draw_text(t4, btn_x + btn_w / 2.0 - d4.width / 2.0, settings_y + btn_h / 2.0 + 7.0, 22.0, WHITE);
            }

            LobbyMode::Settings => {
                let text = "Settings (Coming Soon)";
                let dims = measure_text(text, None, 28, 1.0);
                draw_text(text, ARENA_W / 2.0 - dims.width / 2.0, ARENA_H / 2.0, 28.0, LIGHTGRAY);
                draw_text("Press Escape to go back", ARENA_W / 2.0 - 100.0, ARENA_H / 2.0 + 40.0, 14.0, DARKGRAY);
            }

            LobbyMode::MatchSettings { ref next_action } => {
                let next = next_action.clone();
                let left_click = is_mouse_button_pressed(MouseButton::Left);
                if crate::settings::draw_settings_panel(game_settings, mouse, left_click) {
                    match next {
                        MatchSettingsNext::CreateRoom => {
                            self.room_code = Self::generate_room_code();
                            self.net = Some(NetState::new(&self.room_code));
                            self.status = format!("Room: {}  --  Share this code!", self.room_code);
                            self.mode = LobbyMode::WaitingForPeer;
                        }
                        MatchSettingsNext::VsAi => {
                            return LobbyResult::StartVsAi;
                        }
                    }
                }
                draw_text("Press Escape to go back", ARENA_W / 2.0 - 90.0, ARENA_H - 30.0, 14.0, DARKGRAY);
            }

            LobbyMode::EnteringCode => {
                let label = "Enter Room Code:";
                let ldims = measure_text(label, None, 22, 1.0);
                draw_text(label, ARENA_W / 2.0 - ldims.width / 2.0, ARENA_H / 2.0 - 20.0, 22.0, LIGHTGRAY);

                let code_display = if self.input_code.is_empty() {
                    "____".to_string()
                } else {
                    let mut s = self.input_code.clone();
                    while s.len() < 4 { s.push('_'); }
                    s
                };
                let cdims = measure_text(&code_display, None, 48, 1.0);
                draw_text(&code_display, ARENA_W / 2.0 - cdims.width / 2.0, ARENA_H / 2.0 + 30.0, 48.0, Color::new(0.3, 0.8, 1.0, 1.0));

                if self.input_code.len() == 4 {
                    let connect_x = ARENA_W / 2.0 - 80.0;
                    let connect_y = ARENA_H / 2.0 + 55.0;
                    let cbw = 160.0;
                    let cbh = 40.0;
                    let hover = mouse.x >= connect_x && mouse.x <= connect_x + cbw && mouse.y >= connect_y && mouse.y <= connect_y + cbh;
                    let bg = if hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
                    draw_rectangle(connect_x, connect_y, cbw, cbh, bg);
                    draw_rectangle_lines(connect_x, connect_y, cbw, cbh, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                    let ct = "Connect";
                    let cd = measure_text(ct, None, 20, 1.0);
                    draw_text(ct, connect_x + cbw / 2.0 - cd.width / 2.0, connect_y + cbh / 2.0 + 6.0, 20.0, WHITE);
                }

                draw_text("Press Escape to go back", ARENA_W / 2.0 - 100.0, ARENA_H / 2.0 + 120.0, 14.0, DARKGRAY);
            }

            LobbyMode::WaitingForPeer | LobbyMode::Connected => {
                let code_text = format!("Room: {}", self.room_code);
                let cdims = measure_text(&code_text, None, 36, 1.0);
                draw_text(&code_text, ARENA_W / 2.0 - cdims.width / 2.0, ARENA_H / 2.0 - 10.0, 36.0, Color::new(0.3, 0.8, 1.0, 1.0));

                let sdims = measure_text(&self.status, None, 20, 1.0);
                draw_text(&self.status, ARENA_W / 2.0 - sdims.width / 2.0, ARENA_H / 2.0 + 25.0, 20.0, LIGHTGRAY);

                if self.mode == LobbyMode::WaitingForPeer {
                    let dots = ".".repeat(((get_time() * 2.0) as usize % 4));
                    let wait_text = format!("Waiting for opponent{}", dots);
                    let wdims = measure_text(&wait_text, None, 18, 1.0);
                    draw_text(&wait_text, ARENA_W / 2.0 - wdims.width / 2.0, ARENA_H / 2.0 + 55.0, 18.0, Color::new(0.6, 0.6, 0.6, 1.0));
                }

                draw_text("Press Escape to cancel", ARENA_W / 2.0 - 90.0, ARENA_H / 2.0 + 100.0, 14.0, DARKGRAY);
            }
        }

        LobbyResult::Waiting
    }
}
