use std::pin::Pin;
use std::future::Future;

use matchbox_socket::{PeerId, PeerState, WebRtcSocket};
use serde::{Serialize, Deserialize};

use crate::tech::TechId;
use crate::unit::UnitKind;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    ReadyToStart,
    SettingsSync(crate::settings::GameSettings),
    BuildComplete {
        new_packs: Vec<(usize, (f32, f32), bool)>,
        tech_purchases: Vec<(UnitKind, TechId)>,
        gold_remaining: u32,
    },
    ChatMessage(String, String), // (sender_name, text)
    Surrender,
    RematchRequest,
    BanSelection(Vec<u8>),
    ColorChoice(u8),
    NameSync(String),
    RoundEnd {
        winner: Option<u8>,
        lp_damage: i32,
        loser_team: Option<u8>,
        // Debug checksums for desync detection
        alive_0: u16,
        alive_1: u16,
        total_hp_0: i32,
        total_hp_1: i32,
        // Mutual damage values for timeout rounds (both sides take damage)
        timeout_dmg_0: i32,
        timeout_dmg_1: i32,
    },
    /// Host sends state hash to guest every SYNC_INTERVAL frames
    StateHash { frame: u32, hash: u64 },
    /// Guest requests full state from host when hash mismatch detected
    StateRequest { frame: u32 },
    /// Host sends full authoritative state to guest
    StateSync {
        frame: u32,
        units_data: Vec<u8>,
        projectiles_data: Vec<u8>,
        obstacles_data: Vec<u8>,
    },
}

#[derive(Clone, Debug)]
pub struct OpponentBuildData {
    pub new_packs: Vec<(usize, (f32, f32), bool)>,
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
    pub opponent_build: Option<OpponentBuildData>,
    pub disconnected: bool,
    pub received_chats: Vec<(String, String)>, // (sender_name, text)
    pub opponent_surrendered: bool,
    pub opponent_rematch: bool,
    pub opponent_bans: Option<Vec<u8>>,
    pub received_settings: Option<crate::settings::GameSettings>,
    pub opponent_color: Option<u8>,
    pub opponent_name: Option<String>,
    pub received_round_end: Option<RoundEndData>,
    // Desync detection & state sync
    pub received_state_hash: Option<(u32, u64)>,
    pub received_state_request: Option<u32>,
    pub received_state_sync: Option<StateSyncData>,
}

#[derive(Clone, Debug)]
pub struct RoundEndData {
    pub winner: Option<u8>,
    pub lp_damage: i32,
    pub loser_team: Option<u8>,
    pub alive_0: u16,
    pub alive_1: u16,
    pub timeout_dmg_0: i32,
    pub timeout_dmg_1: i32,
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
            opponent_build: None,
            disconnected: false,
            received_chats: Vec::new(),
            opponent_surrendered: false,
            opponent_rematch: false,
            opponent_bans: None,
            received_settings: None,
            opponent_color: None,
            opponent_name: None,
            received_round_end: None,
            received_state_hash: None,
            received_state_request: None,
            received_state_sync: None,
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
                        NetMessage::BuildComplete {
                            new_packs,
                            tech_purchases,
                            gold_remaining: _,
                        } => {
                            self.opponent_build = Some(OpponentBuildData {
                                new_packs,
                                tech_purchases,
                            });
                        }
                        NetMessage::ChatMessage(name, text) => {
                            self.received_chats.push((name, text));
                        }
                        NetMessage::Surrender => {
                            self.opponent_surrendered = true;
                        }
                        NetMessage::RematchRequest => {
                            self.opponent_rematch = true;
                        }
                        NetMessage::BanSelection(bans) => {
                            self.opponent_bans = Some(bans);
                        }
                        NetMessage::SettingsSync(settings) => {
                            self.received_settings = Some(settings);
                        }
                        NetMessage::ColorChoice(idx) => {
                            self.opponent_color = Some(idx);
                        }
                        NetMessage::NameSync(name) => {
                            self.opponent_name = Some(name);
                        }
                        NetMessage::RoundEnd { winner, lp_damage, loser_team, alive_0, alive_1, total_hp_0: _, total_hp_1: _, timeout_dmg_0, timeout_dmg_1 } => {
                            self.received_round_end = Some(RoundEndData {
                                winner, lp_damage, loser_team, alive_0, alive_1, timeout_dmg_0, timeout_dmg_1,
                            });
                        }
                        NetMessage::StateHash { frame, hash } => {
                            self.received_state_hash = Some((frame, hash));
                        }
                        NetMessage::StateRequest { frame } => {
                            self.received_state_request = Some(frame);
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

    pub fn take_opponent_build(&mut self) -> Option<OpponentBuildData> {
        self.opponent_build.take()
    }
}
