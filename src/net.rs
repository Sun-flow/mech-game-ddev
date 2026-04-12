use std::pin::Pin;
use std::future::Future;

use matchbox_socket::{PeerId, PeerState, WebRtcSocket};
use serde::{Serialize, Deserialize};

use crate::game_state::BuildState;
use crate::tech::TechId;
use crate::unit::UnitKind;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    ReadyToStart,
    SettingsSync(crate::settings::GameSettings),
    BuildComplete {
        player_id: u16,
        /// Each entry: (pack_index, (center_x, center_y), rotated, unit_ids).
        /// unit_ids are the IDs assigned by the spawning client — the receiver
        /// MUST use these exact IDs rather than generating new ones, because
        /// the sender's next_id may have gaps from sold/undone packs.
        new_packs: Vec<(usize, (f32, f32), bool, Vec<u64>)>,
        tech_purchases: Vec<(UnitKind, TechId)>,
        gold_remaining: u32,
    },
    ChatMessage { player_id: u16, name: String, text: String },
    Surrender { player_id: u16 },
    RematchRequest { player_id: u16 },
    BanSelection(Vec<u8>),
    ColorChoice { player_id: u16, color_index: u8 },
    NameSync { player_id: u16, name: String },
    RoundEnd {
        winner: Option<u16>,
        lp_damage: i32,
        loser: Option<u16>,
        per_player: Vec<RoundEndPlayerData>,
    },
    /// Both peers send their state hash every frame. Used for desync detection.
    StateHash { frame: u32, hash: u64 },
    /// Host proactively sends full authoritative state to guest on mismatch
    /// (no request needed — host detects via guest's incoming hash).
    StateSync {
        frame: u32,
        units_data: Vec<u8>,
        projectiles_data: Vec<u8>,
        obstacles_data: Vec<u8>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundEndPlayerData {
    pub player_id: u16,
    pub alive_count: u16,
    pub total_hp: i32,
    pub timeout_damage: i32,
}

#[derive(Clone, Debug)]
pub struct PeerBuildData {
    pub player_id: u16,
    pub new_packs: Vec<(usize, (f32, f32), bool, Vec<u64>)>,
    pub tech_purchases: Vec<(UnitKind, TechId)>,
}

#[derive(Clone, Debug)]
pub struct StateSyncData {
    pub frame: u32,
    pub units_data: Vec<u8>,
    pub projectiles_data: Vec<u8>,
    pub obstacles_data: Vec<u8>,
}

pub struct NetState {
    pub socket: WebRtcSocket,
    pub message_loop: Pin<Box<dyn Future<Output = Result<(), matchbox_socket::Error>>>>,
    pub peer_id: Option<PeerId>,
    pub is_host: bool,
    pub peer_ready: bool,
    pub peer_build: Option<PeerBuildData>,
    pub disconnected: bool,
    pub received_chats: Vec<(u16, String, String)>, // (player_id, sender_name, text)
    pub surrendered_player: Option<u16>,
    pub rematch_player: Option<u16>,
    pub peer_bans: Option<Vec<u8>>,
    pub received_settings: Option<crate::settings::GameSettings>,
    pub peer_color: Option<(u16, u8)>, // (player_id, color_index)
    pub peer_name: Option<(u16, String)>, // (player_id, name)
    pub received_round_end: Option<RoundEndData>,
    // Desync detection & state sync
    pub received_state_hashes: Vec<(u32, u64)>,
    pub received_state_sync: Option<StateSyncData>,
    pub local_player_id: u16,
}

#[derive(Clone, Debug)]
pub struct RoundEndData {
    pub winner: Option<u16>,
    pub lp_damage: i32,
    pub loser: Option<u16>,
    pub per_player: Vec<RoundEndPlayerData>,
}

impl NetState {
    pub fn new(room_code: &str) -> Self {
        let url = format!("wss://match-0-7.helsing.studio/{}?next=2", room_code);
        let (socket, loop_fut) = WebRtcSocket::builder(&url)
            .add_reliable_channel()
            .build();

        Self {
            socket,
            message_loop: Box::pin(loop_fut),
            peer_id: None,
            is_host: false,
            peer_ready: false,
            peer_build: None,
            disconnected: false,
            received_chats: Vec::new(),
            surrendered_player: None,
            rematch_player: None,
            peer_bans: None,
            received_settings: None,
            peer_color: None,
            peer_name: None,
            received_round_end: None,
            received_state_hashes: Vec::new(),
            received_state_sync: None,
            local_player_id: 0,
        }
    }

    /// Call every frame to drive the WebRTC connection.
    pub fn poll(&mut self) {
        // Drive the message loop future (non-blocking)
        let _ = futures_lite::future::block_on(futures_lite::future::poll_once(&mut self.message_loop));

        // Check for new peers - returns Vec<(PeerId, PeerState)>
        if let Ok(new_peers) = self.socket.try_update_peers() {
            for (id, state) in new_peers {
                match state {
                    PeerState::Connected => {
                        self.peer_id = Some(id);
                    }
                    PeerState::Disconnected => {
                        if self.peer_id == Some(id) {
                            self.disconnected = true;
                            self.peer_id = None;
                        }
                    }
                }
            }
        }

        // Receive messages on the reliable channel
        if let Ok(channel) = self.socket.get_channel_mut(0) {
            let messages = channel.receive();
            for (_from, data) in messages {
                match bincode::deserialize::<NetMessage>(&data) {
                    Ok(msg) => match msg {
                        NetMessage::ReadyToStart => {
                            self.peer_ready = true;
                        }
                        NetMessage::BuildComplete { player_id, new_packs, tech_purchases, gold_remaining: _ } => {
                            self.peer_build = Some(PeerBuildData {
                                player_id,
                                new_packs,
                                tech_purchases,
                            });
                        }
                        NetMessage::ChatMessage { player_id, name, text } => {
                            self.received_chats.push((player_id, name, text));
                        }
                        NetMessage::Surrender { player_id } => {
                            self.surrendered_player = Some(player_id);
                        }
                        NetMessage::RematchRequest { player_id } => {
                            self.rematch_player = Some(player_id);
                        }
                        NetMessage::BanSelection(bans) => {
                            self.peer_bans = Some(bans);
                        }
                        NetMessage::SettingsSync(settings) => {
                            self.received_settings = Some(settings);
                        }
                        NetMessage::ColorChoice { player_id, color_index } => {
                            self.peer_color = Some((player_id, color_index));
                        }
                        NetMessage::NameSync { player_id, name } => {
                            self.peer_name = Some((player_id, name));
                        }
                        NetMessage::RoundEnd { winner, lp_damage, loser, per_player } => {
                            self.received_round_end = Some(RoundEndData {
                                winner, lp_damage, loser, per_player,
                            });
                        }
                        NetMessage::StateHash { frame, hash } => {
                            self.received_state_hashes.push((frame, hash));
                        }
                        NetMessage::StateSync { frame, units_data, projectiles_data, obstacles_data } => {
                            self.received_state_sync = Some(StateSyncData {
                                frame, units_data, projectiles_data, obstacles_data,
                            });
                        }
                    },
                    Err(e) => {
                        eprintln!("[NET] Failed to deserialize message ({} bytes): {}", data.len(), e);
                    }
                }
            }
        }
    }

    pub fn send(&mut self, msg: NetMessage) {
        let peer = match self.peer_id {
            Some(p) => p,
            None => {
                eprintln!("[NET] Cannot send: no peer connected");
                return;
            }
        };
        let data = match bincode::serialize(&msg) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[NET] Failed to serialize message: {}", e);
                return;
            }
        };
        match self.socket.get_channel_mut(0) {
            Ok(channel) => channel.send(data.into_boxed_slice(), peer),
            Err(e) => eprintln!("[NET] Failed to get channel: {}", e),
        }
    }

    pub fn is_peer_connected(&self) -> bool {
        self.peer_id.is_some()
    }

    pub fn take_peer_build(&mut self) -> Option<PeerBuildData> {
        self.peer_build.take()
    }

    pub fn derive_local_player_id(&mut self) -> Option<u16> {
        self.socket.id().map(|pid| player_id_from_peer(&pid))
    }
}

/// Send BuildComplete message over the network with this round's new packs and tech purchases.
pub fn send_build_complete(
    net: &mut Option<NetState>,
    build: &BuildState,
    local_player_id: u16,
) {
    if let Some(ref mut n) = net {
        let new_packs: Vec<(usize, (f32, f32), bool, Vec<u64>)> = build
            .placed_packs
            .iter()
            .filter(|p| !p.locked)
            .map(|p| (p.pack_index, (p.center.x, p.center.y), p.rotated, p.unit_ids.clone()))
            .collect();

        let tech_purchases = build.round_tech_purchases.clone();

        n.send(NetMessage::BuildComplete {
            player_id: local_player_id,
            new_packs,
            tech_purchases,
            gold_remaining: build.gold_remaining,
        });
    }
}

/// Derive a u16 player_id from a matchbox PeerId (UUID).
pub fn player_id_from_peer(peer_id: &PeerId) -> u16 {
    let bytes = peer_id.0.as_bytes();
    u16::from_be_bytes([bytes[0], bytes[1]])
}
