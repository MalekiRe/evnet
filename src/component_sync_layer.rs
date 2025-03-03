use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use crate::{Peer, PeerDisconnected, Reliability};
use bevy::ecs::component::{ComponentHooks, StorageType};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Component)]
pub struct DespawnOnDisconnect(pub Peer);

pub struct GeneralComponentSyncPlugin;
impl Plugin for GeneralComponentSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            |mut commands: Commands,
             mut event_reader: EventReader<PeerDisconnected>,
             query: Query<(Entity, &DespawnOnDisconnect)>| {
                for ev in event_reader.read() {
                    for (e, despawn_on_disconnect) in query.iter() {
                        if ev.0 == despawn_on_disconnect.0 {
                            commands.entity(e).despawn_recursive();
                        }
                    }
                }
            },
        );
    }
}

pub struct One;
pub struct Two;
pub struct Three;
pub struct Four;

pub struct ComponentSyncPlugin<C, ReliabilityImplementor, Marker>(
    PhantomData<(C, ReliabilityImplementor, Marker)>,
);
impl<Marker, C, ReliabilityImplementor> Default
    for ComponentSyncPlugin<C, ReliabilityImplementor, Marker>
{
    fn default() -> Self {
        Self(PhantomData)
    }
}
unsafe impl<Marker, C, ReliabilityImplementor> Send
    for ComponentSyncPlugin<C, ReliabilityImplementor, Marker>
{
}
unsafe impl<Marker, C, ReliabilityImplementor> Sync
    for ComponentSyncPlugin<C, ReliabilityImplementor, Marker>
{
}

impl<
    ReliabilityImplementor: NetworkMessage + Send + Sync + 'static,
    C0: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
> Plugin for ComponentSyncPlugin<C0, ReliabilityImplementor, One>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkEntityMapper>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<ReliabilityImplementor, C0>>,
             mut commands: Commands,
             entity_mapper: Res<NetworkEntityMapper>,
             mut query: Query<(&mut NetworkId, &mut Authority, &mut C0)>| {
                for (
                    SyncMsg {
                        network_id,
                        authority,
                        data,
                        phantom_data: _,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&network_id) else {
                        continue;
                    };
                    let Ok((mut _sync_net2, mut authority2, mut c0)) = query.get_mut(*e) else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet>();
                        *authority2 = authority;
                    }
                    *c0 = data;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<ReliabilityImplementor, C0>>,
             query: Query<
                (&NetworkId, &Authority, &C0),
                (Or<(Changed<Authority>, Changed<C0>)>, With<LocalNet>),
            >| {
                for (network_id, authority, c0) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            network_id: *network_id,
                            authority: *authority,
                            data: c0.clone(),
                            phantom_data: PhantomData,
                        },
                        SendType::AllButSelf,
                    )) {
                        error!("{}", err);
                    }
                }
            },
        );
    }
}

impl<
    ReliabilityImplementor: NetworkMessage + Send + Sync + 'static,
    C0: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C1: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
> Plugin for ComponentSyncPlugin<(C0, C1), ReliabilityImplementor, Two>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkEntityMapper>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<ReliabilityImplementor, (C0, C1)>>,
             mut commands: Commands,
             entity_mapper: Res<NetworkEntityMapper>,
             mut query: Query<(&mut NetworkId, &mut Authority, &mut C0, &mut C1)>| {
                for (
                    SyncMsg {
                        network_id,
                        authority,
                        data,
                        phantom_data: _,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&network_id) else {
                        continue;
                    };
                    let Ok((mut _sync_net2, mut authority2, mut c0, mut c1)) = query.get_mut(*e)
                    else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet>();
                        *authority2 = authority;
                    }
                    *c0 = data.0;
                    *c1 = data.1;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<ReliabilityImplementor, (C0, C1)>>,
             query: Query<
                (&NetworkId, &Authority, &C0, &C1),
                (
                    Or<(Changed<Authority>, Changed<C0>, Changed<C1>)>,
                    With<LocalNet>,
                ),
            >| {
                for (network_id, authority, c0, c1) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            network_id: *network_id,
                            authority: *authority,
                            data: (c0.clone(), c1.clone()),
                            phantom_data: PhantomData,
                        },
                        SendType::AllButSelf,
                    )) {
                        error!("{}", err);
                    }
                }
            },
        );
    }
}

impl<
    ReliabilityImplementor: NetworkMessage + Send + Sync + 'static,
    C0: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C1: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C2: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
> Plugin for ComponentSyncPlugin<(C0, C1, C2), ReliabilityImplementor, Three>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkEntityMapper>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<ReliabilityImplementor, (C0, C1, C2)>>,
             mut commands: Commands,
             entity_mapper: Res<NetworkEntityMapper>,
             mut query: Query<(
                 &mut NetworkId,
                 &mut Authority,
                 &mut C0,
                 &mut C1,
                 &mut C2,
             )>| {
                for (
                    SyncMsg {
                        network_id,
                        authority,
                        data,
                        phantom_data: _,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&network_id) else {
                        continue;
                    };
                    let Ok((_sync_net2, mut authority2, mut c0, mut c1, mut c2)) =
                        query.get_mut(*e)
                    else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet>();
                        *authority2 = authority;
                    }
                    *c0 = data.0;
                    *c1 = data.1;
                    *c2 = data.2;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<ReliabilityImplementor, (C0, C1, C2)>>,
             query: Query<
                (&NetworkId, &Authority, &C0, &C1, &C2),
                (
                    Or<(Changed<Authority>, Changed<C0>, Changed<C1>, Changed<C2>)>,
                    With<LocalNet>,
                ),
            >| {
                for (network_id, authority, c0, c1, c2) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            network_id: *network_id,
                            authority: *authority,
                            data: (c0.clone(), c1.clone(), c2.clone()),
                            phantom_data: PhantomData,
                        },
                        SendType::AllButSelf,
                    )) {
                        error!("{}", err);
                    }
                }
            },
        );
    }
}

impl<
    ReliabilityImplementor: NetworkMessage + Send + Sync + 'static,
    C0: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C1: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C2: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C3: Send + Sync + Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
> Plugin for ComponentSyncPlugin<(C0, C1, C2, C3), ReliabilityImplementor, Four>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkEntityMapper>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<ReliabilityImplementor, (C0, C1, C2, C3)>>,
             mut commands: Commands,
             entity_mapper: Res<NetworkEntityMapper>,
             mut query: Query<(
                &mut NetworkId,
                &mut Authority,
                &mut C0,
                &mut C1,
                &mut C2,
                &mut C3,
            )>| {
                for (
                    SyncMsg {
                        network_id,
                        authority,
                        data,
                        phantom_data: _,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&network_id) else {
                        continue;
                    };
                    let Ok((_sync_net2, mut authority2, mut c0, mut c1, mut c2, mut c3)) =
                        query.get_mut(*e)
                    else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet>();
                        *authority2 = authority;
                    }
                    *c0 = data.0;
                    *c1 = data.1;
                    *c2 = data.2;
                    *c3 = data.3;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<ReliabilityImplementor, (C0, C1, C2, C3)>>,
             query: Query<
                (&NetworkId, &Authority, &C0, &C1, &C2, &C3),
                (
                    Or<(
                        Changed<Authority>,
                        Changed<C0>,
                        Changed<C1>,
                        Changed<C2>,
                        Changed<C3>,
                    )>,
                    With<LocalNet>,
                ),
            >| {
                for (network_id, authority, c0, c1, c2, c3) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            network_id: *network_id,
                            authority: *authority,
                            data: (c0.clone(), c1.clone(), c2.clone(), c3.clone()),
                            phantom_data: PhantomData,
                        },
                        SendType::AllButSelf,
                    )) {
                        error!("{}", err);
                    }
                }
            },
        );
    }
}

#[derive(Serialize, Deserialize)]
pub struct SyncMsg<ReliabilityImplementor: NetworkMessage, Data: Send + Sync + 'static> {
    network_id: NetworkId,
    authority: Authority,
    data: Data,
    phantom_data: PhantomData<ReliabilityImplementor>,
}

impl<
    ReliabilityImplementor: NetworkMessage,
    Data: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
> NetworkMessage for SyncMsg<ReliabilityImplementor, Data>
{
    const RELIABILITY: Reliability = ReliabilityImplementor::RELIABILITY;
}
#[derive(Deref, DerefMut, Resource, Default)]
pub struct NetworkEntityMapper(pub HashMap<NetworkId, Entity>);
pub struct LocalNet;
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Hash, PartialOrd, PartialEq, Eq, Ord)]
pub struct NetworkId(u32, Peer);
impl NetworkId {
    pub fn new<'w>(me: &crate::Me<'w>) -> Self {
        Self(random_number::random!(), me.get())
    }
}

#[derive(
    Component,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Default,
)]
pub struct Authority(u32);
impl Component for LocalNet {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world: DeferredWorld, targeted_entity, _component_id| {
            let mut awa = world.entity_mut(targeted_entity);
            let mut uwu: Mut<Authority> = awa
                .get_mut::<Authority>()
                .expect("should have Authority<T>");
            uwu.0 += 1;
        });
    }
}

impl bevy::ecs::component::Component for NetworkId
where
    Self: Send + Sync + 'static,
{
    const STORAGE_TYPE: bevy::ecs::component::StorageType =
        bevy::ecs::component::StorageType::Table;
    #[allow(unused_variables)]
    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_add(|mut world: DeferredWorld, targeted_entity, _component_id| {
            let awa = world.entity(targeted_entity);
            let thing = awa.get::<NetworkId>().unwrap().clone();
            world
                .get_resource_mut::<NetworkEntityMapper>()
                .unwrap()
                .0
                .insert(thing, targeted_entity);
        });
    }
    fn register_required_components(
        requiree: bevy::ecs::component::ComponentId,
        components: &mut bevy::ecs::component::Components,
        storages: &mut bevy::ecs::storage::Storages,
        required_components: &mut bevy::ecs::component::RequiredComponents,
        inheritance_depth: u16,
    ) {
        components.register_required_components_manual::<Self, Authority>(
            storages,
            required_components,
            Authority::default,
            inheritance_depth,
        );
        <Authority as bevy::ecs::component::Component>::register_required_components(
            requiree,
            components,
            storages,
            required_components,
            inheritance_depth + 1,
        );
    }
}
