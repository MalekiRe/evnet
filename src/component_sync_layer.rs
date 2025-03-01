use crate::{Peer, PeerDisconnected, Reliability};
use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use bevy::ecs::component::{ComponentHooks, StorageType};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

#[derive(Component)]
pub struct DespawnOnDisconnect(pub Peer);

pub struct GeneralComponentSyncPlugin;
impl Plugin for GeneralComponentSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, |mut commands: Commands, mut event_reader: EventReader<PeerDisconnected>, query: Query<(Entity, &DespawnOnDisconnect)>| {
            for ev in event_reader.read() {
                for (e, despawn_on_disconnect) in query.iter() {
                    if ev.0 == despawn_on_disconnect.0 {
                        commands.entity(e).despawn_recursive();
                    }
                }
            }
        });
    }
}

pub struct One;
pub struct Two;
pub struct Three;
pub struct Four;

pub struct ComponentSyncPlugin<C, Name, Marker>(PhantomData<(C, Name, Marker)>);
impl<Marker, C, Name> Default for ComponentSyncPlugin<C, Name, Marker> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
unsafe impl<Marker, C, Name> Send for ComponentSyncPlugin<C, Name, Marker> {}
unsafe impl<Marker, C, Name> Sync for ComponentSyncPlugin<C, Name, Marker> {}
impl<
    C: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    Name: NetworkMessage + Serialize + for<'de> Deserialize<'de> + 'static,
> Plugin for ComponentSyncPlugin<C, Name, One>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMapper<SyncNet<Name>>>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<Name, C>>,
             mut commands: Commands,
             entity_mapper: Res<EntityMapper<SyncNet<Name>>>,
             mut query: Query<(&mut SyncNet<Name>, &mut Authority<Name>, &mut C)>| {
                for (
                    SyncMsg {
                        sync_net,
                        authority,
                        data,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&sync_net) else {
                        continue;
                    };
                    let Ok((mut sync_net2, mut authority2, mut data2)) = query.get_mut(*e) else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet<Name>>();
                        *authority2 = authority;
                    }
                    *data2 = data;
                    *sync_net2 = sync_net;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<Name, C>>,
             query: Query<
                (&SyncNet<Name>, &Authority<Name>, &C),
                (
                    Or<(Changed<Authority<Name>>, Changed<C>)>,
                    With<LocalNet<Name>>,
                ),
            >| {
                for (sync_net, authority, data) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            sync_net: *sync_net,
                            authority: *authority,
                            data: data.clone(),
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
    C0: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C1: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    Name: NetworkMessage + Serialize + for<'de> Deserialize<'de> + 'static,
> Plugin for ComponentSyncPlugin<(C0, C1), Name, Two>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMapper<SyncNet<Name>>>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<Name, (C0, C1)>>,
             mut commands: Commands,
             entity_mapper: Res<EntityMapper<SyncNet<Name>>>,
             mut query: Query<(&mut SyncNet<Name>, &mut Authority<Name>, &mut C0, &mut C1)>| {
                for (
                    SyncMsg {
                        sync_net,
                        authority,
                        data,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&sync_net) else {
                        continue;
                    };
                    let Ok((mut sync_net2, mut authority2, mut c0, mut c1)) = query.get_mut(*e) else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet<Name>>();
                        *authority2 = authority;
                    }
                    *c0 = data.0;
                    *c1 = data.1;
                    *sync_net2 = sync_net;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<Name, (C0, C1)>>,
             query: Query<
                (&SyncNet<Name>, &Authority<Name>, &C0, &C1),
                (
                    Or<(Changed<Authority<Name>>, Changed<C0>, Changed<C1>)>,
                    With<LocalNet<Name>>,
                ),
            >| {
                for (sync_net, authority, c0, c1) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            sync_net: *sync_net,
                            authority: *authority,
                            data: (c0.clone(), c1.clone()),
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
    C0: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C1: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C2: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    Name: NetworkMessage + Serialize + for<'de> Deserialize<'de> + 'static,
> Plugin for ComponentSyncPlugin<(C0, C1, C2), Name, Three>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMapper<SyncNet<Name>>>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<Name, (C0, C1, C2)>>,
             mut commands: Commands,
             entity_mapper: Res<EntityMapper<SyncNet<Name>>>,
             mut query: Query<(
                &mut SyncNet<Name>,
                &mut Authority<Name>,
                &mut C0,
                &mut C1,
                &mut C2,
            )>| {
                for (
                    SyncMsg {
                        sync_net,
                        authority,
                        data,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&sync_net) else {
                        continue;
                    };
                    let Ok((mut sync_net2, mut authority2, mut c0, mut c1, mut c2)) =
                        query.get_mut(*e)
                    else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet<Name>>();
                        *authority2 = authority;
                    }
                    *c0 = data.0;
                    *c1 = data.1;
                    *c2 = data.2;
                    *sync_net2 = sync_net;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<Name, (C0, C1, C2)>>,
             query: Query<
                (&SyncNet<Name>, &Authority<Name>, &C0, &C1, &C2),
                (
                    Or<(
                        Changed<Authority<Name>>,
                        Changed<C0>,
                        Changed<C1>,
                        Changed<C2>,
                    )>,
                    With<LocalNet<Name>>,
                ),
            >| {
                for (sync_net, authority, c0, c1, c2) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            sync_net: *sync_net,
                            authority: *authority,
                            data: (c0.clone(), c1.clone(), c2.clone()),
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
    C0: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C1: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C2: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    C3: Component + Serialize + for<'de> Deserialize<'de> + Clone + 'static,
    Name: NetworkMessage + Serialize + for<'de> Deserialize<'de> + 'static,
> Plugin for ComponentSyncPlugin<(C0, C1, C2, C3), Name, Four>
{
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMapper<SyncNet<Name>>>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<Name, (C0, C1, C2, C3)>>,
             mut commands: Commands,
             entity_mapper: Res<EntityMapper<SyncNet<Name>>>,
             mut query: Query<(
                &mut SyncNet<Name>,
                &mut Authority<Name>,
                &mut C0,
                &mut C1,
                &mut C2,
                &mut C3,
            )>| {
                for (
                    SyncMsg {
                        sync_net,
                        authority,
                        data,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&sync_net) else {
                        continue;
                    };
                    let Ok((mut sync_net2, mut authority2, mut c0, mut c1, mut c2, mut c3)) =
                        query.get_mut(*e)
                    else {
                        continue;
                    };
                    if authority.0 > authority2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet<Name>>();
                        *authority2 = authority;
                    }
                    *c0 = data.0;
                    *c1 = data.1;
                    *c2 = data.2;
                    *c3 = data.3;
                    *sync_net2 = sync_net;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<SyncMsg<Name, (C0, C1, C2, C3)>>,
             query: Query<
                (&SyncNet<Name>, &Authority<Name>, &C0, &C1, &C2, &C3),
                (
                    Or<(
                        Changed<Authority<Name>>,
                        Changed<C0>,
                        Changed<C1>,
                        Changed<C2>,
                        Changed<C3>,
                    )>,
                    With<LocalNet<Name>>,
                ),
            >| {
                for (sync_net, authority, c0, c1, c2, c3) in query.iter() {
                    if let Err(err) = sender.send((
                        SyncMsg {
                            sync_net: *sync_net,
                            authority: *authority,
                            data: (c0.clone(), c1.clone(), c2.clone(), c3.clone()),
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
pub struct SyncMsg<Name: 'static, Data: Send + Sync + 'static> {
    sync_net: SyncNet<Name>,
    authority: Authority<Name>,
    data: Data,
}

impl<
    T: NetworkMessage + Serialize + for<'de> Deserialize<'de> + 'static,
    Data: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
> NetworkMessage for SyncMsg<T, Data>
{
    const RELIABILITY: Reliability = T::RELIABILITY;
}
unsafe impl<
    T: Serialize + for<'de> Deserialize<'de> + 'static,
    Data: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
> Send for SyncMsg<T, Data>
{
}
unsafe impl<
    T: Serialize + for<'de> Deserialize<'de> + 'static,
    Data: Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static,
> Sync for SyncMsg<T, Data>
{
}
#[derive(Deref, DerefMut, Resource)]
pub struct EntityMapper<T: Hash>(pub HashMap<T, Entity>);
impl<T: Hash> Default for EntityMapper<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}
pub struct LocalNet<T: 'static>(PhantomData<T>);
impl<T: 'static> Default for LocalNet<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
unsafe impl<T> Send for LocalNet<T> {}
unsafe impl<T> Sync for LocalNet<T> {}
#[derive(Serialize, Deserialize)]
pub struct SyncNet<T: 'static>(u32, PhantomData<T>);
unsafe impl<T> Send for SyncNet<T> {}
unsafe impl<T> Sync for SyncNet<T> {}
impl<T> SyncNet<T> {
    pub fn new() -> Self {
        Self(random_number::random!(), PhantomData)
    }
}
impl<T: 'static> Clone for SyncNet<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<T: 'static> Copy for SyncNet<T> {}
impl<T: 'static> Hash for SyncNet<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl<T: 'static> PartialEq for SyncNet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: 'static> Eq for SyncNet<T> {}
#[derive(Component, Serialize, Deserialize)]
pub struct Authority<T: 'static>(u32, PhantomData<T>);
impl<T: 'static> Clone for Authority<T> {
    fn clone(&self) -> Self {
        Authority(self.0, PhantomData)
    }
}
impl<T: 'static> Copy for Authority<T> {}
unsafe impl<T> Send for Authority<T> {}
unsafe impl<T> Sync for Authority<T> {}
impl<T: 'static> Default for Authority<T> {
    fn default() -> Self {
        Self(0, PhantomData)
    }
}
impl<T: 'static> Component for LocalNet<T> {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world: DeferredWorld, targeted_entity, _component_id| {
            let mut awa = world.entity_mut(targeted_entity);
            let mut uwu: Mut<Authority<T>> = awa
                .get_mut::<Authority<T>>()
                .expect("should have Authority<T>");
            uwu.0 += 1;
        });
    }
}

impl<T: 'static> bevy::ecs::component::Component for SyncNet<T>
where
    Self: Send + Sync + 'static,
{
    const STORAGE_TYPE: bevy::ecs::component::StorageType =
        bevy::ecs::component::StorageType::Table;
    #[allow(unused_variables)]
    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_add(|mut world: DeferredWorld, targeted_entity, _component_id| {
            let awa = world.entity(targeted_entity);
            let thing = awa.get::<SyncNet<T>>().unwrap().clone();
            world
                .get_resource_mut::<EntityMapper<SyncNet<T>>>()
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
        components.register_required_components_manual::<Self, Authority<T>>(
            storages,
            required_components,
            <Authority<T> as Default>::default,
            inheritance_depth,
        );
        <Authority<T> as bevy::ecs::component::Component>::register_required_components(
            requiree,
            components,
            storages,
            required_components,
            inheritance_depth + 1,
        );
    }
}
