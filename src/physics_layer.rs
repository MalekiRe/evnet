use crate::Reliability;
use crate::component_sync_layer::{ComponentSyncPlugin, Four};
use crate::message_layer::NetworkMessage;
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Physics;
impl NetworkMessage for Physics {
    const RELIABILITY: Reliability = Reliability::UnreliableOrdered;
}

pub type PhysicsSyncPlugin =
    ComponentSyncPlugin<(Position, Rotation, LinearVelocity, AngularVelocity), Physics, Four>;
