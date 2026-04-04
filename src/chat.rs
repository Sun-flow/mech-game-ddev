use macroquad::prelude::*;

use crate::game_state::GamePhase;
use crate::net;
use crate::team;

pub struct ChatMessage {
    pub name: String,
    pub text: String,
    pub team_id: u8,
    pub lifetime: f32,
}

pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub open: bool,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            open: false,
        }
    }

    /// Receive incoming chat messages from network.
    pub fn receive_from_net(&mut self, net: &mut Option<net::NetState>) {
        if let Some(ref mut n) = net {
            for (name, text) in n.received_chats.drain(..) {
                self.messages.push(ChatMessage {
                    name,
                    text,
                    team_id: 1,
                    lifetime: 5.0,
                });
            }
        }
    }

    /// Handle chat input and sending.
    pub fn update(
        &mut self,
        phase: &GamePhase,
        net: &mut Option<net::NetState>,
        player_name: &str,
    ) {
        let chat_allowed = matches!(
            phase,
            GamePhase::Build | GamePhase::Battle | GamePhase::RoundResult { .. }
        );
        if !chat_allowed {
            return;
        }

        if is_key_pressed(KeyCode::Enter) {
            if self.open {
                if !self.input.is_empty() {
                    let text = if self.input.len() > 100 {
                        self.input[..100].to_string()
                    } else {
                        self.input.clone()
                    };
                    self.messages.push(ChatMessage {
                        name: player_name.to_string(),
                        text: text.clone(),
                        team_id: 0,
                        lifetime: 5.0,
                    });
                    if let Some(ref mut n) = net {
                        n.send(net::NetMessage::ChatMessage(player_name.to_string(), text));
                    }
                }
                self.input.clear();
                self.open = false;
            } else {
                self.open = true;
            }
        }

        if self.open {
            if is_key_pressed(KeyCode::Escape) {
                self.open = false;
                self.input.clear();
            }
            while let Some(ch) = get_char_pressed() {
                if ch == '\r' || ch == '\n' {
                    continue;
                }
                if ch == '\u{8}' {
                    self.input.pop();
                } else if self.input.len() < 100 && (ch.is_ascii_graphic() || ch == ' ') {
                    self.input.push(ch);
                }
            }
        }
    }

    /// Update lifetimes and remove expired messages.
    pub fn tick(&mut self, dt: f32) {
        for msg in self.messages.iter_mut() {
            msg.lifetime -= dt;
        }
        self.messages.retain(|m| m.lifetime > 0.0);
    }

    /// Draw chat messages and input box.
    pub fn draw(&self, phase: &GamePhase, player_name: &str) {
        // Chat messages
        let chat_x = screen_width() / 2.0;
        let mut chat_y = crate::ui::s(45.0);
        for msg in self
            .messages
            .iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            let alpha = (msg.lifetime / 5.0).min(1.0);
            let color = team::team_color(msg.team_id);
            let display_color = Color::new(color.r, color.g, color.b, alpha);

            let is_emote = msg.text.starts_with('/');
            let display_text = match msg.text.as_str() {
                "/gg" => "GG".to_string(),
                "/gl" => "Good Luck!".to_string(),
                "/nice" => "Nice!".to_string(),
                "/wow" => "Wow!".to_string(),
                _ => msg.text.clone(),
            };
            let full_display = format!("{}: {}", msg.name, display_text);
            let font_size = if is_emote { 20.0 } else { 15.0 };
            let dims = crate::ui::measure_scaled_text(&full_display, font_size as u16);
            crate::ui::draw_scaled_text(
                &full_display,
                chat_x - dims.width / 2.0,
                chat_y,
                font_size,
                display_color,
            );
            chat_y += font_size + 4.0;
        }

        // Input box
        if self.open {
            let input_y = screen_height() - crate::ui::s(45.0);
            let input_w = crate::ui::s(450.0);
            let input_x = screen_width() / 2.0 - input_w / 2.0;
            let input_h = crate::ui::s(30.0);
            draw_rectangle(
                input_x,
                input_y,
                input_w,
                input_h,
                Color::new(0.05, 0.05, 0.1, 0.92),
            );
            draw_rectangle_lines(
                input_x,
                input_y,
                input_w,
                input_h,
                1.5,
                Color::new(0.4, 0.5, 0.6, 0.9),
            );
            let name_prefix = format!("{}: ", player_name);
            let name_w = crate::ui::measure_scaled_text(&name_prefix, 15).width;
            crate::ui::draw_scaled_text(
                &name_prefix,
                input_x + 8.0,
                input_y + 20.0,
                15.0,
                Color::new(0.6, 0.8, 1.0, 0.9),
            );
            let cursor = if (get_time() * 2.0) as u32 % 2 == 0 {
                "|"
            } else {
                ""
            };
            crate::ui::draw_scaled_text(
                &format!("{}{}", self.input, cursor),
                input_x + 8.0 + name_w,
                input_y + 20.0,
                15.0,
                WHITE,
            );
        } else {
            let chat_allowed = matches!(
                phase,
                GamePhase::Build | GamePhase::Battle | GamePhase::RoundResult { .. }
            );
            if chat_allowed {
                crate::ui::draw_scaled_text(
                    "Enter: Chat",
                    screen_width() - crate::ui::s(100.0),
                    screen_height() - crate::ui::s(5.0),
                    12.0,
                    Color::new(0.4, 0.4, 0.4, 0.6),
                );
            }
        }
    }
}
