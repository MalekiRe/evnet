pub mod component_sync_layer;
pub mod event_layer;
pub mod message_layer;
pub mod physics_layer;

use crate::message_layer::{AppExt, MessageReceiver, NetworkMessage, incoming};
use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::ecs::system::SystemParam;
use bevy::prelude::{
    Commands, Component, DetectChanges, FromWorld, In, IntoSystem, IntoSystemConfigs, Local, Res,
    ResMut, Resource, Startup, Update, World, not, resource_exists, resource_removed,
};
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::prelude::PeerId;
use flume::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

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

pub struct BaseNetworkingPlugin;

impl Plugin for BaseNetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (|mut commands: Commands, mut socket: ResMut<MatchboxSocket>| {
                let Some(id) = socket.id() else { return };
                commands.insert_resource(Me(id.into()));
            })
            .run_if(not(resource_exists::<Me>)),
        );
    }
}

pub struct NetworkingPlugins;

impl PluginGroup for NetworkingPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BaseNetworkingPlugin)
            .add(message_layer::MessageLayerPlugin)
    }
}

#[derive(Component, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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
pub struct Me(Peer);

impl Me {
    pub fn get(&self) -> Peer {
        self.0
    }
}

impl PartialEq<Peer> for Me {
    fn eq(&self, other: &Peer) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Me> for Peer {
    fn eq(&self, other: &Me) -> bool {
        self.eq(&other.0)
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
    pub fn try_new(val: usize) -> Option<Self> {
        match val {
            RELIABLE => Some(Reliability::Reliable),
            UNRELIABLE => Some(Reliability::Unreliable),
            UNRELIABLE_ORDERED => Some(Reliability::UnreliableOrdered),
            _ => None,
        }
    }
}
