use macroquad::prelude::*;

use crate::net::NetState;
use crate::ui::s;

// Helper to get current screen dimensions for UI layout
fn sw() -> f32 { screen_width() }
fn sh() -> f32 { screen_height() }

#[derive(PartialEq)]
pub enum LobbyMode {
    Menu,
    Settings,
    MatchSettings { next_action: MatchSettingsNext },
    EnteringCode,
    WaitingForPeer,
    Connected,
    ColorPick,
}

#[derive(Clone, Debug, PartialEq)]
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
    pub is_room_creator: bool,
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
            is_room_creator: false,
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

    pub fn update(&mut self, game_settings: &mut crate::settings::GameSettings, _main_settings: &mut crate::settings::MainSettings, ms: &crate::input::MouseState) -> LobbyResult {
        let mouse = ms.screen_mouse;
        let left_click = ms.left_click;

        let btn_w = s(240.0);
        let btn_h = s(45.0);
        let btn_x = sw() / 2.0 - btn_w / 2.0;

        match self.mode {
            LobbyMode::Menu => {
                // Name field editing
                let name_y = sh() / 2.0 - s(80.0);
                let name_w = s(200.0);
                let name_x = sw() / 2.0 - name_w / 2.0;
                let name_h = s(30.0);
                let name_hovered = mouse.x >= name_x && mouse.x <= name_x + name_w
                    && mouse.y >= name_y && mouse.y <= name_y + name_h;
                if left_click && name_hovered {
                    self.name_editing = true;
                } else if left_click {
                    self.name_editing = false;
                }
                if ms.right_click && name_hovered {
                    self.player_name.clear();
                    self.name_editing = true;
                }
                if self.name_editing {
                    while let Some(ch) = get_char_pressed() {
                        if ch == '\u{8}' { self.player_name.pop(); }
                        else if self.player_name.len() < 16 && (ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ' ') {
                            self.player_name.push(ch);
                        }
                    }
                }

                let create_y = sh() / 2.0 - s(25.0);

                // "Create Room" → go to Match Settings
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= create_y && mouse.y <= create_y + btn_h
                {
                    self.mode = LobbyMode::MatchSettings { next_action: MatchSettingsNext::CreateRoom };
                }

                // "Join Room" button
                let join_y = create_y + s(55.0);
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= join_y && mouse.y <= join_y + btn_h
                {
                    self.input_code.clear();
                    self.mode = LobbyMode::EnteringCode;
                }

                // "Play vs AI" → go to Match Settings
                let ai_y = join_y + s(55.0);
                if left_click
                    && mouse.x >= btn_x && mouse.x <= btn_x + btn_w
                    && mouse.y >= ai_y && mouse.y <= ai_y + btn_h
                {
                    self.mode = LobbyMode::MatchSettings { next_action: MatchSettingsNext::VsAi };
                }

                // "Settings" button (placeholder)
                let settings_y = ai_y + s(55.0);
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
                    let connect_x = sw() / 2.0 - 80.0;
                    let connect_y = sh() / 2.0 + 50.0;
                    let cbw = s(160.0);
                    let cbh = s(40.0);

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
                        // Derive player ID from PeerId
                        let my_pid = net.derive_local_player_id().unwrap_or(macroquad::rand::gen_range(1000, 60000));
                        net.local_player_id = my_pid;

                        if self.is_room_creator {
                            net.send(crate::net::NetMessage::SettingsSync(game_settings.clone()));
                        }
                        net.send(crate::net::NetMessage::NameSync { player_id: my_pid, name: self.player_name.clone() });
                        net.send(crate::net::NetMessage::ColorChoice { player_id: my_pid, color_index: game_settings.player_color_index });
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

                    // Joiner: apply host's match settings and go to color pick
                    if !self.is_room_creator {
                        if let Some(settings) = net.received_settings.take() {
                            let peer_color_idx = net.peer_color.map(|(_, c)| c).unwrap_or(255);
                            let my_color = game_settings.player_color_index;
                            *game_settings = settings;
                            game_settings.player_color_index = my_color;
                            // If peer color matches ours, pick a different one
                            if my_color == peer_color_idx {
                                for i in 0..6u8 {
                                    if i != peer_color_idx {
                                        game_settings.player_color_index = i;
                                        break;
                                    }
                                }
                            }
                            self.mode = LobbyMode::ColorPick;
                            return LobbyResult::Waiting;
                        }
                    } else {
                        // Creator: discard any settings received (guest may have sent defaults)
                        net.received_settings.take();
                    }

                    if net.peer_ready {
                        return LobbyResult::StartMultiplayer;
                    }
                }
            }

            LobbyMode::ColorPick => {
                // Handled in draw()
                if let Some(ref mut net) = self.net {
                    net.poll();
                }
            }
        }

        LobbyResult::Waiting
    }

    pub fn draw(&mut self, game_settings: &mut crate::settings::GameSettings, main_settings: &mut crate::settings::MainSettings, ms: &crate::input::MouseState) -> LobbyResult {
        clear_background(Color::new(0.08, 0.08, 0.1, 1.0));

        let title = "RTS Unit Arena";
        let tdims = crate::ui::measure_scaled_text(title, 40);
        crate::ui::draw_scaled_text(title, sw() / 2.0 - tdims.width / 2.0, sh() / 2.0 - s(140.0), 40.0, WHITE);

        let subtitle = "Multiplayer";
        let sdims = crate::ui::measure_scaled_text(subtitle, 24);
        crate::ui::draw_scaled_text(subtitle, sw() / 2.0 - sdims.width / 2.0, sh() / 2.0 - s(105.0), 24.0, Color::new(0.5, 0.7, 1.0, 1.0));

        let mouse = ms.screen_mouse;
        let btn_w = s(240.0);
        let btn_h = s(45.0);
        let btn_x = sw() / 2.0 - btn_w / 2.0;

        match self.mode {
            LobbyMode::Menu => {
                // Player name field
                let name_y = sh() / 2.0 - s(80.0);
                let name_w = s(200.0);
                let name_x = sw() / 2.0 - name_w / 2.0;
                let name_h = s(30.0);
                crate::ui::draw_scaled_text("Name:", name_x - s(50.0), name_y + s(20.0), 16.0, LIGHTGRAY);
                let name_bg = if self.name_editing { Color::new(0.15, 0.15, 0.22, 0.95) } else { Color::new(0.1, 0.1, 0.15, 0.8) };
                draw_rectangle(name_x, name_y, name_w, name_h, name_bg);
                let border_color = if self.name_editing { Color::new(0.4, 0.7, 1.0, 1.0) } else { Color::new(0.3, 0.3, 0.4, 0.8) };
                draw_rectangle_lines(name_x, name_y, name_w, name_h, 1.0, border_color);
                let cursor = if self.name_editing && ((get_time() * 2.0) as u32).is_multiple_of(2) { "|" } else { "" };
                crate::ui::draw_scaled_text(&format!("{}{}", self.player_name, cursor), name_x + s(6.0), name_y + s(20.0), 16.0, WHITE);

                let create_y = sh() / 2.0 - s(25.0);

                // Create Room
                let hover = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= create_y && mouse.y <= create_y + btn_h;
                let bg = if hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
                draw_rectangle(btn_x, create_y, btn_w, btn_h, bg);
                draw_rectangle_lines(btn_x, create_y, btn_w, btn_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                let t = "Create Room";
                let d = crate::ui::measure_scaled_text(t, 22);
                crate::ui::draw_scaled_text(t, btn_x + btn_w / 2.0 - d.width / 2.0, create_y + btn_h / 2.0 + s(7.0), 22.0, WHITE);

                // Join Room
                let join_y = create_y + s(55.0);
                let hover2 = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= join_y && mouse.y <= join_y + btn_h;
                let bg2 = if hover2 { Color::new(0.2, 0.3, 0.5, 0.9) } else { Color::new(0.15, 0.2, 0.35, 0.8) };
                draw_rectangle(btn_x, join_y, btn_w, btn_h, bg2);
                draw_rectangle_lines(btn_x, join_y, btn_w, btn_h, 2.0, Color::new(0.3, 0.5, 0.9, 1.0));
                let t2 = "Join Room";
                let d2 = crate::ui::measure_scaled_text(t2, 22);
                crate::ui::draw_scaled_text(t2, btn_x + btn_w / 2.0 - d2.width / 2.0, join_y + btn_h / 2.0 + s(7.0), 22.0, WHITE);

                // Play vs AI
                let ai_y = join_y + s(55.0);
                let hover3 = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= ai_y && mouse.y <= ai_y + btn_h;
                let bg3 = if hover3 { Color::new(0.4, 0.3, 0.2, 0.9) } else { Color::new(0.3, 0.2, 0.15, 0.8) };
                draw_rectangle(btn_x, ai_y, btn_w, btn_h, bg3);
                draw_rectangle_lines(btn_x, ai_y, btn_w, btn_h, 2.0, Color::new(0.9, 0.6, 0.3, 1.0));
                let t3 = "Play vs AI";
                let d3 = crate::ui::measure_scaled_text(t3, 22);
                crate::ui::draw_scaled_text(t3, btn_x + btn_w / 2.0 - d3.width / 2.0, ai_y + btn_h / 2.0 + s(7.0), 22.0, WHITE);

                // Settings
                let settings_y = ai_y + s(55.0);
                let hover4 = mouse.x >= btn_x && mouse.x <= btn_x + btn_w && mouse.y >= settings_y && mouse.y <= settings_y + btn_h;
                let bg4 = if hover4 { Color::new(0.3, 0.3, 0.35, 0.9) } else { Color::new(0.2, 0.2, 0.25, 0.8) };
                draw_rectangle(btn_x, settings_y, btn_w, btn_h, bg4);
                draw_rectangle_lines(btn_x, settings_y, btn_w, btn_h, 2.0, Color::new(0.6, 0.6, 0.7, 1.0));
                let t4 = "Settings";
                let d4 = crate::ui::measure_scaled_text(t4, 22);
                crate::ui::draw_scaled_text(t4, btn_x + btn_w / 2.0 - d4.width / 2.0, settings_y + btn_h / 2.0 + s(7.0), 22.0, WHITE);
            }

            LobbyMode::Settings => {
                let panel_w = s(400.0);
                let panel_h = s(150.0);
                let px = sw() / 2.0 - panel_w / 2.0;
                let py = sh() / 2.0 - panel_h / 2.0;
                draw_rectangle(px, py, panel_w, panel_h, Color::new(0.1, 0.1, 0.15, 0.95));
                draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, Color::new(0.4, 0.4, 0.5, 1.0));

                let title = "Settings";
                let tdims = crate::ui::measure_scaled_text(title, 24);
                crate::ui::draw_scaled_text(title, px + panel_w / 2.0 - tdims.width / 2.0, py + 30.0, 24.0, WHITE);

                crate::settings::draw_ui_scale_slider(main_settings, ms.screen_mouse, ms.left_click, ms.left_down, px, py + 55.0);

                crate::ui::draw_scaled_text("Press Escape to go back", sw() / 2.0 - 100.0, py + panel_h + 20.0, 14.0, DARKGRAY);
            }

            LobbyMode::MatchSettings { ref next_action } => {
                let next = next_action.clone();
                if crate::settings::draw_settings_panel(game_settings, ms.screen_mouse, ms.left_click) {
                    match next {
                        MatchSettingsNext::CreateRoom => {
                            self.is_room_creator = true;
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
                crate::ui::draw_scaled_text("Press Escape to go back", sw() / 2.0 - 90.0, sh() - s(30.0), 14.0, DARKGRAY);
            }

            LobbyMode::EnteringCode => {
                let label = "Enter Room Code:";
                let ldims = crate::ui::measure_scaled_text(label, 22);
                crate::ui::draw_scaled_text(label, sw() / 2.0 - ldims.width / 2.0, sh() / 2.0 - 20.0, 22.0, LIGHTGRAY);

                let code_display = if self.input_code.is_empty() {
                    "____".to_string()
                } else {
                    // Pad with underscores to show remaining chars needed
                    format!("{:_<4}", self.input_code)
                };
                let cdims = crate::ui::measure_scaled_text(&code_display, 48);
                crate::ui::draw_scaled_text(&code_display, sw() / 2.0 - cdims.width / 2.0, sh() / 2.0 + 30.0, 48.0, Color::new(0.3, 0.8, 1.0, 1.0));

                if self.input_code.len() == 4 {
                    let connect_x = sw() / 2.0 - 80.0;
                    let connect_y = sh() / 2.0 + s(55.0);
                    let cbw = s(160.0);
                    let cbh = s(40.0);
                    let hover = mouse.x >= connect_x && mouse.x <= connect_x + cbw && mouse.y >= connect_y && mouse.y <= connect_y + cbh;
                    let bg = if hover { Color::new(0.2, 0.5, 0.3, 0.9) } else { Color::new(0.15, 0.35, 0.2, 0.8) };
                    draw_rectangle(connect_x, connect_y, cbw, cbh, bg);
                    draw_rectangle_lines(connect_x, connect_y, cbw, cbh, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                    let ct = "Connect";
                    let cd = crate::ui::measure_scaled_text(ct, 20);
                    crate::ui::draw_scaled_text(ct, connect_x + cbw / 2.0 - cd.width / 2.0, connect_y + cbh / 2.0 + 6.0, 20.0, WHITE);
                }

                crate::ui::draw_scaled_text("Press Escape to go back", sw() / 2.0 - 100.0, sh() / 2.0 + s(120.0), 14.0, DARKGRAY);
            }

            LobbyMode::WaitingForPeer | LobbyMode::Connected => {
                let code_text = format!("Room: {}", self.room_code);
                let cdims = crate::ui::measure_scaled_text(&code_text, 36);
                crate::ui::draw_scaled_text(&code_text, sw() / 2.0 - cdims.width / 2.0, sh() / 2.0 - 10.0, 36.0, Color::new(0.3, 0.8, 1.0, 1.0));

                let sdims = crate::ui::measure_scaled_text(&self.status, 20);
                crate::ui::draw_scaled_text(&self.status, sw() / 2.0 - sdims.width / 2.0, sh() / 2.0 + 25.0, 20.0, LIGHTGRAY);

                if self.mode == LobbyMode::WaitingForPeer {
                    let dots = ".".repeat((get_time() * 2.0) as usize % 4);
                    let wait_text = format!("Waiting for opponent{}", dots);
                    let wdims = crate::ui::measure_scaled_text(&wait_text, 18);
                    crate::ui::draw_scaled_text(&wait_text, sw() / 2.0 - wdims.width / 2.0, sh() / 2.0 + s(55.0), 18.0, Color::new(0.6, 0.6, 0.6, 1.0));
                }

                crate::ui::draw_scaled_text("Press Escape to cancel", sw() / 2.0 - 90.0, sh() / 2.0 + s(100.0), 14.0, DARKGRAY);
            }

            LobbyMode::ColorPick => {
                let left_click = ms.left_click;
                let peer_color_idx = if let Some(ref n) = self.net { n.peer_color.map(|(_, c)| c).unwrap_or(255) } else { 255 };

                let pick_title = "Choose Your Team Color";
                let ptdims = crate::ui::measure_scaled_text(pick_title, 28);
                crate::ui::draw_scaled_text(pick_title, sw() / 2.0 - ptdims.width / 2.0, sh() / 2.0 - 60.0, 28.0, WHITE);

                let swatch_size = s(50.0);
                let swatch_gap = s(16.0);
                let sy = sh() / 2.0 - 20.0;

                if let Some(color_idx) = crate::settings::draw_color_swatches(
                    game_settings.player_color_index, mouse, left_click,
                    sw() / 2.0, sy, swatch_size, swatch_gap, Some(peer_color_idx),
                ) {
                    game_settings.player_color_index = color_idx;
                }

                // Ready button
                let rbtn_w = s(200.0);
                let rbtn_h = s(45.0);
                let rbtn_x = sw() / 2.0 - rbtn_w / 2.0;
                let rbtn_y = sy + swatch_size + 45.0;
                let rbtn_hover = mouse.x >= rbtn_x && mouse.x <= rbtn_x + rbtn_w && mouse.y >= rbtn_y && mouse.y <= rbtn_y + rbtn_h;
                let rbtn_bg = if rbtn_hover { Color::new(0.2, 0.6, 0.3, 0.9) } else { Color::new(0.15, 0.45, 0.2, 0.8) };
                draw_rectangle(rbtn_x, rbtn_y, rbtn_w, rbtn_h, rbtn_bg);
                draw_rectangle_lines(rbtn_x, rbtn_y, rbtn_w, rbtn_h, 2.0, Color::new(0.3, 0.8, 0.4, 1.0));
                let rt = "Ready";
                let rdims = crate::ui::measure_scaled_text(rt, 22);
                crate::ui::draw_scaled_text(rt, rbtn_x + rbtn_w / 2.0 - rdims.width / 2.0, rbtn_y + rbtn_h / 2.0 + s(7.0), 22.0, WHITE);

                if left_click && rbtn_hover {
                    // Send our color choice to the host
                    if let Some(ref mut net) = self.net {
                        let my_pid = net.local_player_id;
                        net.send(crate::net::NetMessage::ColorChoice { player_id: my_pid, color_index: game_settings.player_color_index });
                    }
                    return LobbyResult::StartMultiplayer;
                }
            }
        }

        LobbyResult::Waiting
    }
}
