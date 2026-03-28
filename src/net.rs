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
    RoundEnd {
        winner: Option<u8>,
        lp_damage: i32,
        loser_team: Option<u8>,
        // Debug checksums for desync detection
        alive_0: u16,
        alive_1: u16,
        total_hp_0: i32,
        total_hp_1: i32,
    },
}

#[derive(Clone, Debug)]
pub struct OpponentBuildData {
    pub new_packs: Vec<(usize, (f32, f32), bool)>,
    pub tech_purchases: Vec<(UnitKind, TechId)>,
    pub gold_remaining: u32,
}

pub struct NetState {
    pub socket: WebRtcSocket,
    pub message_loop: Pin<Box<dyn Future<Output = Result<(), matchbox_socket::Error>>>>,
    pub peer_id: Option<PeerId>,
    pub is_host: bool,
    pub peer_ready: bool,
    pub opponent_build: Option<OpponentBuildData>,
    pub local_ready: bool,
    pub disconnected: bool,
    pub received_chats: Vec<(String, String)>, // (sender_name, text)
    pub opponent_surrendered: bool,
    pub opponent_rematch: bool,
    pub opponent_bans: Option<Vec<u8>>,
    pub received_settings: Option<crate::settings::GameSettings>,
    pub opponent_color: Option<u8>,
    pub received_round_end: Option<RoundEndData>,
}

#[derive(Clone, Debug)]
pub struct RoundEndData {
    pub winner: Option<u8>,
    pub lp_damage: i32,
    pub loser_team: Option<u8>,
    pub alive_0: u16,
    pub alive_1: u16,
    pub total_hp_0: i32,
    pub total_hp_1: i32,
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
            local_ready: false,
            disconnected: false,
            received_chats: Vec::new(),
            opponent_surrendered: false,
            opponent_rematch: false,
            opponent_bans: None,
            received_settings: None,
            opponent_color: None,
            received_round_end: None,
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
                        if self.peer_id.is_none() {
                            self.is_host = true;
                        }
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
                if let Ok(msg) = bincode::deserialize::<NetMessage>(&data) {
                    match msg {
                        NetMessage::ReadyToStart => {
                            self.peer_ready = true;
                        }
                        NetMessage::BuildComplete {
                            new_packs,
                            tech_purchases,
                            gold_remaining,
                        } => {
                            self.opponent_build = Some(OpponentBuildData {
                                new_packs,
                                tech_purchases,
                                gold_remaining,
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
                        NetMessage::RoundEnd { winner, lp_damage, loser_team, alive_0, alive_1, total_hp_0, total_hp_1 } => {
                            self.received_round_end = Some(RoundEndData {
                                winner, lp_damage, loser_team, alive_0, alive_1, total_hp_0, total_hp_1,
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn send(&mut self, msg: NetMessage) {
        if let Some(peer) = self.peer_id {
            if let Ok(data) = bincode::serialize(&msg) {
                if let Ok(channel) = self.socket.get_channel_mut(0) {
                    channel.send(data.into_boxed_slice(), peer);
                }
            }
        }
    }

    pub fn is_peer_connected(&self) -> bool {
        self.peer_id.is_some()
    }

    pub fn take_opponent_build(&mut self) -> Option<OpponentBuildData> {
        self.opponent_build.take()
    }
}
