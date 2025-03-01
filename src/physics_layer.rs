use crate::Reliability;
use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::ecs::component::{ComponentHooks, ComponentId, Components, RequiredComponents, StorageType};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use bevy::ecs::storage::Storages;

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize,
)]
pub struct PhysicsSync(u32);

impl bevy::ecs::component::Component for PhysicsSync
where
    Self: Send + Sync + 'static,
{
    const STORAGE_TYPE: bevy::ecs::component::StorageType = bevy::ecs::component::StorageType::Table;
    fn register_required_components(requiree: bevy::ecs::component::ComponentId, components: &mut bevy::ecs::component::Components, storages: &mut bevy::ecs::storage::Storages, required_components: &mut bevy::ecs::component::RequiredComponents, inheritance_depth: u16 ) {
        components.register_required_components_manual::<Self, Ownership>(storages, required_components, <Ownership as Default>::default, inheritance_depth);
        <Ownership as bevy::ecs::component::Component>::register_required_components(requiree, components, storages, required_components, inheritance_depth + 1);
    }
    #[allow(unused_variables)]
    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_add(|mut world: DeferredWorld, targeted_entity, _component_id| {
            let mut awa = world.entity(targeted_entity);
            let thing = *awa.get::<PhysicsSync>().unwrap();
            world.get_resource_mut::<EntityMapper<PhysicsSync>>().unwrap().0.insert(thing, targeted_entity);
        });
    }
}

impl PhysicsSync {
    pub fn new() -> PhysicsSync {
        PhysicsSync(random_number::random!())
    }
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct LocalNet;

impl Component for LocalNet {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world: DeferredWorld, targeted_entity, _component_id| {
            let mut awa = world.entity_mut(targeted_entity);
            let mut uwu: Mut<Ownership> =
                awa.get_mut::<Ownership>().expect("should have ownership");
            uwu.0 += 1;
        });
    }
}

#[derive(
    Component,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Serialize,
    Deserialize,
    Default,
)]
pub struct Ownership(u64);

#[derive(Resource)]
pub struct EntityMapper<T: Hash>(pub HashMap<T, Entity>);

impl<T: Hash> Default for EntityMapper<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhysicsMessage {
    id: PhysicsSync,
    owner: Ownership,
    position: Position,
    rotation: Rotation,
    linear_velocity: LinearVelocity,
    angular_velocity: AngularVelocity,
}
impl NetworkMessage for PhysicsMessage {
    const RELIABILITY: Reliability = Reliability::UnreliableOrdered;
}

#[derive(Event)]
pub struct SpawnNewPhysics {
    physics_message: PhysicsMessage,
}

pub struct PhysicsLayerPlugin;
impl Plugin for PhysicsLayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMapper<PhysicsSync>>();
        app.add_network_message(
            |rx: MessageReceiver<PhysicsMessage>,
             mut commands: Commands,
             mut entity_mapper: ResMut<EntityMapper<PhysicsSync>>,
             mut query: Query<(
                &mut Position,
                &mut Rotation,
                &mut LinearVelocity,
                &mut AngularVelocity,
                &mut Ownership,
            )>| {
                for (
                    PhysicsMessage {
                        id,
                        owner,
                        position,
                        rotation,
                        linear_velocity,
                        angular_velocity,
                    },
                    _peer,
                ) in rx.try_iter()
                {
                    let Some(e) = entity_mapper.0.get(&id) else {
                        continue;
                    };
                    let Ok((
                        mut position2,
                        mut rotation2,
                        mut linear_velocity2,
                        mut angular_velocity2,
                        mut ownership2,
                    )) = query.get_mut(*e) else { continue };
                    if owner.0 > ownership2.as_ref().0 {
                        commands.entity(*e).remove::<LocalNet>();
                        *ownership2 = owner;
                    }
                    *position2 = position;
                    *rotation2 = rotation;
                    *linear_velocity2 = linear_velocity;
                    *angular_velocity2 = angular_velocity;
                }
            },
        );
        app.add_systems(
            PostUpdate,
            |sender: MessageSender<PhysicsMessage>,
             query: Query<
                (
                    &PhysicsSync,
                    &Ownership,
                    &Position,
                    &Rotation,
                    &LinearVelocity,
                    &AngularVelocity,
                ),
                With<LocalNet>,
            >| {
                for (id, owner, position, rotation, linear_velocity, angular_velocity) in
                    query.iter()
                {
                    sender
                        .send((
                            PhysicsMessage {
                                id: *id,
                                owner: *owner,
                                position: *position,
                                rotation: *rotation,
                                linear_velocity: *linear_velocity,
                                angular_velocity: *angular_velocity,
                            },
                            SendType::AllButSelf,
                        ))
                        .unwrap()
                }
            },
        );
    }
}
