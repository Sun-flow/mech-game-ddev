use serde::{Serialize, Deserialize};

/// Game settings configurable in the lobby before starting a match.
/// Gameplay-changing settings are toggleable; visual/UX features are always on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSettings {
    /// Enable terrain obstacles on the battlefield
    pub terrain_enabled: bool,
    /// Allow destructible cover (in addition to indestructible walls)
    pub terrain_destructible: bool,
    /// Enable draft/ban phase where each player bans 2 unit types
    pub draft_ban_enabled: bool,
    /// Enable smarter AI (counter-picking, balanced compositions)
    pub smart_ai: bool,
    /// Player's chosen team color index (0-5)
    pub player_color_index: u8,
}

impl GameSettings {
    pub fn default() -> Self {
        Self {
            terrain_enabled: false,
            terrain_destructible: true,
            draft_ban_enabled: false,
            smart_ai: false,
            player_color_index: 0, // Red by default
        }
    }
}

/// Available team colors for customization
pub const TEAM_COLOR_OPTIONS: &[(& str, (f32, f32, f32))] = &[
    ("Red",    (0.9, 0.2, 0.2)),
    ("Blue",   (0.2, 0.4, 0.9)),
    ("Green",  (0.2, 0.8, 0.3)),
    ("Yellow", (0.9, 0.8, 0.2)),
    ("Purple", (0.7, 0.2, 0.9)),
    ("Orange", (0.9, 0.5, 0.1)),
];
