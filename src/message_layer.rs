use crate::message_layer::outgoing::SenderRes;
use crate::{Me, Peer, RELIABLE, Reliability, UNRELIABLE, UNRELIABLE_ORDERED};
use bevy::ecs::archetype::ArchetypeComponentId;
use bevy::ecs::component::{ComponentId, Tick};
use bevy::ecs::query::Access;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::DeferredWorld;
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy_matchbox::MatchboxSocket;
use flume::Receiver;
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use crate::conditioner::LinkConditioner;

// This is the base layer
pub trait NetworkMessage: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    const RELIABILITY: Reliability;
}

#[derive(Serialize, Deserialize)]
pub struct MessageWrapper {
    pub type_id_hash: u32,
    pub content: Vec<u8>,
}
impl MessageWrapper {
    pub fn _new<T: Serialize + 'static>(content: &T) -> Self {
        Self {
            type_id_hash: Self::hash::<T>(),
            content: bincode::serialize(content).unwrap(),
        }
    }
    pub fn serialize<T: Serialize + 'static>(content: &T) -> Vec<u8> {
        bincode::serialize(&Self::_new(content)).unwrap()
    }
    pub fn hash<T: Serialize + 'static>() -> u32 {
        let mut hasher = DefaultHasher::new();
        type_name::<T>().hash(&mut hasher);
        hasher.finish() as u32
    }
}

pub enum SendType {
    All,
    AllButSelf,
    Many(Vec<Peer>),
    One(Peer),
}

pub mod outgoing {
    pub type Sender<Message> = flume::Sender<(Message, crate::message_layer::SendType)>;
    pub type Receiver<Message> = flume::Receiver<(Message, crate::message_layer::SendType)>;
    #[derive(bevy::prelude::Resource)]
    pub struct SenderRes<Message: Send + Sync + 'static>(pub Sender<Message>);
}
pub mod incoming {
    pub type Sender<Message> = flume::Sender<(Message, crate::Peer)>;
    pub type Receiver<Message> = flume::Receiver<(Message, crate::Peer)>;
}

pub struct MessageReceiver<'a, Message: Send>(pub &'a mut Receiver<(Message, Peer)>);

impl<Message: Send + 'static> SystemInput for MessageReceiver<'_, Message> {
    type Param<'i> = MessageReceiver<'i, Message>;
    type Inner<'i> = &'i mut Receiver<(Message, Peer)>;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        MessageReceiver(this)
    }
}

impl<'i, T: Send + 'static> Deref for MessageReceiver<'i, T> {
    type Target = Receiver<(T, Peer)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'i, T: Send + 'static> DerefMut for MessageReceiver<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(SystemParam)]
pub struct MessageSender<'w, Message: Send + Sync + 'static>(
    pub ResMut<'w, outgoing::SenderRes<Message>>,
);
impl<'w, Message: Send + Sync + 'static> Deref for MessageSender<'w, Message> {
    type Target = outgoing::Sender<Message>;

    fn deref(&self) -> &Self::Target {
        &self.0.deref().0
    }
}
impl<'w, Message: Send + Sync + 'static> DerefMut for MessageSender<'w, Message> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.deref_mut().0
    }
}

#[derive(Resource, Default)]
pub struct MessageRouter {
    pub route_incoming_messages: HashMap<u32, Box<dyn FnMut(Vec<u8>, Peer) + Send + Sync + 'static>>,
    pub route_outgoing_messages: Vec<
        Box<dyn Fn(&mut MatchboxSocket, Me, &[matchbox_socket::PeerId]) + Send + Sync + 'static>,
    >,
}

pub struct MessageLayerPlugin;
impl Plugin for MessageLayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, route_messages.run_if(resource_exists::<Me>));
    }
}

pub(crate) fn route_messages(world: &mut World) {
    let Some(me) = world.get_resource::<Me>() else {
        return;
    };
    let me = *me;
    let _ = world
        .get_resource_mut::<MatchboxSocket>()
        .unwrap()
        .update_peers();
    let peers = world
        .get_resource_mut::<MatchboxSocket>()
        .unwrap()
        .connected_peers()
        .collect::<Vec<_>>();
    world.resource_scope(|world, mut networked_messages: Mut<MessageRouter>| {
        world.resource_scope(|_world, mut socket: Mut<MatchboxSocket>| {
            for (peer_id, msg) in socket
                .channel_mut(RELIABLE)
                .receive()
                .into_iter()
                .chain(socket.channel_mut(UNRELIABLE).receive())
                .chain(socket.channel_mut(UNRELIABLE_ORDERED).receive())
                .map(|(peer_id, msg)| {
                    (
                        peer_id,
                        bincode::deserialize::<MessageWrapper>(&msg).unwrap(),
                    )
                })
            {
                let route_incoming_messages = networked_messages
                    .route_incoming_messages
                    .get_mut(&msg.type_id_hash)
                    .unwrap();
                route_incoming_messages(msg.content, peer_id.into());
            }
            for route_outgoing_messages in &networked_messages.route_outgoing_messages {
                route_outgoing_messages(&mut socket, me, &peers);
            }
        });
    });
}

pub trait AppExt {
    fn add_network_message<Message: NetworkMessage + Sync + Send + 'static, M>(
        &mut self,
        handler: impl IntoSystem<MessageReceiver<'static, Message>, (), M>,
    );
}
impl AppExt for App {
    fn add_network_message<Message: NetworkMessage + Sync + Send + 'static, M>(
        &mut self,
        handler: impl IntoSystem<MessageReceiver<'static, Message>, (), M>,
    ) {
        self.init_resource::<MessageRouter>();
        let (outgoing_tx, outgoing_rx): (outgoing::Sender<Message>, outgoing::Receiver<Message>) =
            flume::unbounded();
        let (incoming_tx, incoming_rx): (incoming::Sender<Message>, incoming::Receiver<Message>) =
            flume::unbounded();
        let input_wrapper = LocalInputWrapper(IntoSystem::into_system(handler), incoming_rx);
        self.add_systems(Update, input_wrapper);
        let incoming_tx_2 = incoming_tx.clone();
        self.insert_resource(SenderRes(outgoing_tx));
        if let Some(mut conditioner) = self.world_mut().remove_resource::<LinkConditioner<Message>>() {
            self.world_mut()
                .resource_mut::<MessageRouter>()
                .route_incoming_messages
                .insert(
                    MessageWrapper::hash::<Message>(),
                    Box::new(move |bytes: Vec<u8>, peer: Peer| {
                        conditioner.condition_packet(bytes, peer);
                        let mut v = vec![];
                        while let Some((bytes, peer)) = conditioner.pop_packet() {
                            v.push((bytes, peer));
                        }
                        for (bytes, peer) in v.into_iter() {
                            incoming_tx
                                .send((bincode::deserialize(&bytes).unwrap(), peer))
                                .unwrap();
                        }
                    }),
                );
        } else {
            self.world_mut()
                .resource_mut::<MessageRouter>()
                .route_incoming_messages
                .insert(
                    MessageWrapper::hash::<Message>(),
                    Box::new(move |bytes: Vec<u8>, peer: Peer| {
                        incoming_tx
                            .send((bincode::deserialize(&bytes).unwrap(), peer))
                            .unwrap();
                    }),
                );
        }
        self.world_mut()
            .resource_mut::<MessageRouter>()
            .route_outgoing_messages
            .push(Box::new(
                move |socket: &mut MatchboxSocket, me: Me, peers: &[matchbox_socket::PeerId]| {
                    for (message, sender) in outgoing_rx.try_iter() {
                        let channel = socket.channel_mut(Message::RELIABILITY as usize);
                        let msg_bytes = MessageWrapper::serialize(&message);
                        match sender {
                            SendType::All => {
                                for peer in peers {
                                    if let Err(err) =
                                        channel.try_send(msg_bytes.clone().into(), *peer)
                                    {
                                        error!("{}", err);
                                    }
                                }
                                incoming_tx_2.send((message, me.0)).unwrap()
                            }
                            SendType::AllButSelf => {
                                for peer in peers {
                                    if let Err(err) =
                                        channel.try_send(msg_bytes.clone().into(), *peer)
                                    {
                                        error!("{}", err);
                                    }
                                }
                            }
                            SendType::Many(peers) => {
                                for peer in peers {
                                    if let Err(err) =
                                        channel.try_send(msg_bytes.clone().into(), peer.into())
                                    {
                                        error!("{}", err);
                                    }
                                }
                            }
                            SendType::One(peer) => {
                                if let Err(err) =
                                    channel.try_send(msg_bytes.clone().into(), peer.into())
                                {
                                    error!("{}", err);
                                }
                            }
                        }
                    }
                },
            ))
    }
}

struct LocalInputWrapper<S, T>(S, T);

impl<S, T: Send + Sync + 'static> System for LocalInputWrapper<S, Receiver<(T, Peer)>>
where
    S: System<In = MessageReceiver<'static, T>>,
{
    type In = ();

    type Out = S::Out;

    fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        self.0.component_access()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.0.archetype_component_access()
    }

    fn is_send(&self) -> bool {
        self.0.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.0.is_exclusive()
    }

    fn has_deferred(&self) -> bool {
        self.0.has_deferred()
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        world: UnsafeWorldCell,
    ) -> Self::Out {
        unsafe { self.0.run_unsafe(&mut self.1, world) }
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.0.apply_deferred(world)
    }

    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.0.queue_deferred(world)
    }

    unsafe fn validate_param_unsafe(&mut self, world: UnsafeWorldCell) -> bool {
        unsafe { self.0.validate_param_unsafe(world) }
    }

    fn initialize(&mut self, world: &mut World) {
        self.0.initialize(world);
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.0.update_archetype_component_access(world)
    }

    fn check_change_tick(&mut self, change_tick: Tick) {
        self.0.check_change_tick(change_tick)
    }

    fn get_last_run(&self) -> Tick {
        self.0.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.0.set_last_run(last_run)
    }
}
