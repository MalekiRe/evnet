use bevy::app::App;
use bevy::prelude::*;
use bevy_matchbox::matchbox_socket::{ChannelConfig, WebRtcSocket};
use bevy_matchbox::prelude::PeerId;
use bevy_matchbox::MatchboxSocket;
use futures::channel::mpsc::SendError;
use serde::{Deserialize, Serialize};
// Core traits and types
//----------------------------------------

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Reliability {
    Reliable = 0,
    Unreliable = 1,
    UnreliableOrdered = 2,
}
impl Reliability {
    pub const RELIABILITY: [Reliability; 3] = [
        Reliability::Reliable,
        Reliability::Unreliable,
        Reliability::UnreliableOrdered,
    ];
}

pub trait NetworkedEvent {
    const RELIABILITY: Reliability;
    fn id(&self) -> PeerId;
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub type_path_hash: u32,
    pub content: Vec<u8>,
}
impl Message {
    pub fn new<T: Serialize>(content: &T) -> Self {
        Message {
            type_path_hash: const_fnv1a_hash::fnv1a_hash_str_32(std::any::type_name::<T>()),
            content: bincode::serialize(content).unwrap(),
        }
    }
}

// Socket message handling trait and implementation
//----------------------------------------

pub trait SocketSendMessage {
    fn receive_msg(&mut self, reliability: Reliability) -> impl Iterator<Item = (PeerId, Message)>;
    fn send_msg_all<T: Serialize + Event + NetworkedEvent>(
        &mut self,
        message: &T,
        peers: &[PeerId],
    ) -> Result<(), SendError>;
    fn send_msg<T: Serialize + Event + NetworkedEvent>(
        &mut self,
        peer: PeerId,
        message: &T,
    ) -> Result<(), SendError>;
}

impl SocketSendMessage for WebRtcSocket {
    fn receive_msg(&mut self, reliability: Reliability) -> impl Iterator<Item = (PeerId, Message)> {
        self.channel_mut(reliability as usize)
            .receive()
            .into_iter()
            .map(|(id, packet)| (id, bincode::deserialize(&packet).unwrap()))
    }
    fn send_msg_all<T: Serialize + Event + NetworkedEvent>(
        &mut self,
        message: &T,
        peers: &[PeerId],
    ) -> Result<(), SendError> {
        for peer in peers {
            self.send_msg(*peer, message)?;
        }
        Ok(())
    }

    fn send_msg<T: Serialize + Event + NetworkedEvent>(
        &mut self,
        peer: PeerId,
        message: &T,
    ) -> Result<(), SendError> {
        let msg = Message::new(message);
        let msg = bincode::serialize(&msg).unwrap();
        self.channel_mut(T::RELIABILITY as usize)
            .try_send(msg.into(), peer)?;
        Ok(())
    }
}

// Message routing and handling
//----------------------------------------

#[derive(Default, Resource)]
pub struct NetworkedMessages(
    //v Route Incoming    //v Route Outgoing
    std::collections::HashMap<u32, (fn(&mut World, &[u8]), fn(&mut World))>,
);

fn route_outgoing_messages<T: NetworkedEvent + Event + Serialize + for<'a> Deserialize<'a>>(
    world: &mut World,
) {
    world.resource_scope(|world, mut socket: Mut<MatchboxSocket>| {
        let peers = socket.connected_peers().collect::<Vec<_>>();
        let events = world.resource_mut::<Events<T>>();
        let mut cursor = events.get_cursor();
        for e in cursor.read(&events) {
            if e.id() == socket.id().expect("Not connected") {
                if let Err(err) = socket.send_msg_all(e, &peers) {
                    error!("Failed to send message: {:?}", err);
                }
            }
        }
    });
}

fn route_incoming_messages<T: NetworkedEvent + Event + for<'a> Deserialize<'a>>(
    world: &mut World,
    message: &[u8],
) {
    let e: T = bincode::deserialize(message).unwrap();
    world.send_event(e);
}

fn route_messages(world: &mut World) {
    if !world.contains_resource::<MatchboxSocket>() {
        return;
    }

    world.resource_scope(|world, networked_messages: Mut<NetworkedMessages>| {
        world.resource_scope(|world, mut socket: Mut<MatchboxSocket>| {
            for reliability in Reliability::RELIABILITY {
                for (_peer_id, msg) in socket.receive_msg(reliability) {
                    let route_incoming_messages =
                        networked_messages.0.get(&msg.type_path_hash).unwrap().0;
                    route_incoming_messages(world, &msg.content);
                }
            }
        });
        for (_, route_outgoing_messages) in networked_messages.0.values() {
            route_outgoing_messages(world);
        }
    });
    let _peers = world
        .get_resource_mut::<MatchboxSocket>()
        .unwrap()
        .update_peers();
}

#[derive(Clone, Copy, Resource, Default)]
pub struct LocalId(Option<PeerId>);
impl LocalId {
    pub fn get(&self) -> Option<PeerId> {
        self.0
    }
}
impl std::ops::Deref for LocalId {
    type Target = Option<PeerId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn local_id_set(mut local_id: ResMut<LocalId>, matchbox_socket: Option<ResMut<MatchboxSocket>>) {
    if let Some(mut matchbox_socket) = matchbox_socket {
        local_id.0 = matchbox_socket.id();
    } else {
        local_id.0.take();
    }
}

// Plugin implementation and app extension
//----------------------------------------
#[derive(Default)]
pub struct EvnetPlugin {
    url: Option<String>,
}

impl EvnetPlugin {
    pub fn new() -> Self {
        Self { url: None }
    }
    pub fn with_connection(url: impl AsRef<str>) -> Self {
        Self {
            url: Some(url.as_ref().to_string()),
        }
    }
}

impl Plugin for EvnetPlugin {
    fn build(&self, app: &mut App) {
        if let Some(url) = &self.url {
            let url = url.clone();
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.connect(&url);
            });
        }
        app.add_systems(Update, (local_id_set, route_messages).chain());
        app.init_resource::<NetworkedMessages>();
        app.init_resource::<LocalId>();
    }
}

pub trait NetworkedAppExt {
    fn register_networked_event<T: NetworkedEvent + Event + for<'a> Deserialize<'a> + Serialize>(
        &mut self,
    ) -> &mut Self;
}

impl NetworkedAppExt for App {
    fn register_networked_event<T: NetworkedEvent + Event + for<'a> Deserialize<'a> + Serialize>(
        &mut self,
    ) -> &mut Self {
        self.add_event::<T>();
        self.init_resource::<NetworkedMessages>();
        let mut networked_messages = self.world_mut().resource_mut::<NetworkedMessages>();
        networked_messages.0.insert(
            const_fnv1a_hash::fnv1a_hash_str_32(std::any::type_name::<T>()),
            (route_incoming_messages::<T>, route_outgoing_messages::<T>),
        );
        self
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
                .add_channel(ChannelConfig {
                    // UnreliableOrdered
                    ordered: true,
                    max_retransmits: Some(0),
                })
                .build(),
        );
        self.insert_resource(matchbox);
    }
}
