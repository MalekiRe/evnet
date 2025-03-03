pub mod component_sync_layer;
pub mod event_layer;
pub mod message_layer;
pub mod physics_layer;

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::ecs::system::SystemParam;
use bevy::prelude::{
    Commands, Component, Event, EventWriter, IntoSystemConfigs, Local, PreUpdate, Res, ResMut,
    Resource, Update, not, resource_exists,
};
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::prelude::PeerId;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use uuid::Uuid;

#[derive(Event)]
pub struct PeerDisconnected(Peer);
impl PeerDisconnected {
    pub fn get(&self) -> Peer {
        self.0
    }
}
#[derive(Event)]
pub struct PeerConnected(Peer);
impl PeerConnected {
    pub fn get(&self) -> Peer {
        self.0
    }
}

pub trait NetworkedCommandExt {
    fn connect(&mut self, room: &str);
}

impl NetworkedCommandExt for Commands<'_, '_> {
    fn connect(&mut self, room_url: &str) {
        let matchbox = MatchboxSocket::from(
            //example: "wss://mb.v-sekai.cloud/my-room-1"
            bevy_matchbox::matchbox_socket::WebRtcSocketBuilder::new(room_url)
                .add_reliable_channel()
                .add_unreliable_channel()
                .add_channel(matchbox_socket::ChannelConfig {
                    // UnreliableOrdered
                    ordered: true,
                    max_retransmits: Some(0),
                })
                .build(),
        );
        self.insert_resource(matchbox);
    }
}

pub fn connected(me: Option<Res<MeRes>>) -> bool {
    me.is_some()
}

pub struct BaseNetworkingPlugin;

impl Plugin for BaseNetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (|mut commands: Commands, mut socket: ResMut<MatchboxSocket>| {
                let Some(id) = socket.id() else { return };
                commands.insert_resource(MeRes(id.into()));
            })
            .run_if(not(connected)),
        );
        app.add_event::<PeerDisconnected>();
        app.add_event::<PeerConnected>();
        app.add_systems(
            PreUpdate,
            |mut disconnected: Local<Vec<Peer>>,
             mut event_writer: EventWriter<PeerDisconnected>,
             socket: ResMut<MatchboxSocket>| {
                for disconnected_peer in socket.disconnected_peers() {
                    let disconnected_peer: Peer = (*disconnected_peer).into();
                    if !disconnected.contains(&disconnected_peer) {
                        disconnected.push(disconnected_peer);
                        event_writer.send(PeerDisconnected(disconnected_peer));
                    }
                }
            },
        );
        app.add_systems(
            PreUpdate,
            |mut connected: Local<Vec<Peer>>,
             mut event_writer: EventWriter<PeerConnected>,
             socket: ResMut<MatchboxSocket>| {
                for connected_peer in socket.connected_peers() {
                    let connected_peer: Peer = (connected_peer).into();
                    if !connected.contains(&connected_peer) {
                        connected.push(connected_peer);
                        event_writer.send(PeerConnected(connected_peer));
                    }
                }
            },
        );
    }
}

pub struct NetworkingPlugins;

impl PluginGroup for NetworkingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BaseNetworkingPlugin)
            .add(message_layer::MessageLayerPlugin)
            .add(component_sync_layer::GeneralComponentSyncPlugin)
    }
}

#[derive(
    Component, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize,
)]
pub struct Peer(u128);

impl From<Peer> for matchbox_socket::PeerId {
    fn from(value: Peer) -> Self {
        matchbox_socket::PeerId(Uuid::from_u128(value.0))
    }
}
impl From<matchbox_socket::PeerId> for Peer {
    fn from(value: PeerId) -> Self {
        Peer(value.0.as_u128())
    }
}

#[derive(Resource, Component, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MeRes(Peer);

#[derive(SystemParam)]
pub struct Me<'w> {
    me_res: Res<'w, MeRes>,
}
impl PartialEq<Peer> for Me<'_> {
    fn eq(&self, other: &Peer) -> bool {
        self.0 == other.0
    }
}
impl PartialEq<&Peer> for Me<'_> {
    fn eq(&self, other: &&Peer) -> bool {
        self.0 == other.0
    }
}

impl Deref for Me<'_> {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.me_res.0
    }
}

impl Me<'_> {
    pub fn get(&self) -> Peer {
        Peer(self.0)
    }
}

impl PartialEq<Me<'_>> for Peer {
    fn eq(&self, other: &Me) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<Peer> for &Peer {
    fn eq(&self, other: &Peer) -> bool {
        other.0 == self.0
    }
}

pub const RELIABLE: usize = 0;
pub const UNRELIABLE: usize = 1;
pub const UNRELIABLE_ORDERED: usize = 2;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Reliability {
    Reliable,
    Unreliable,
    UnreliableOrdered,
}

impl Reliability {
    pub const fn try_new(val: usize) -> Option<Self> {
        match val {
            RELIABLE => Some(Reliability::Reliable),
            UNRELIABLE => Some(Reliability::Unreliable),
            UNRELIABLE_ORDERED => Some(Reliability::UnreliableOrdered),
            _ => None,
        }
    }
}
