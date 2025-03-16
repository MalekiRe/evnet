use std::marker::PhantomData;
use crate::Reliability;
use crate::component_sync_layer::{Authority, ComponentSyncPlugin, Four, LocalNet, NetworkEntityMapper, NetworkId, SyncMsg};
use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use avian3d::prelude::{AngularVelocity, LinearVelocity, Position, Rotation};
use bevy::app::{App, Plugin};
use bevy::log::error;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Physics;
impl NetworkMessage for Physics {
    const RELIABILITY: Reliability = Reliability::UnreliableOrdered;
}



pub struct PhysicsSyncPlugin;
impl Plugin for PhysicsSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkEntityMapper>();
        app.add_network_message(
            |rx: MessageReceiver<SyncMsg<Physics, (Position, Rotation, LinearVelocity, AngularVelocity)>>,
             mut commands: Commands,
             entity_mapper: Res<NetworkEntityMapper>,
             mut query: Query<(
                 &mut NetworkId,
                 &mut Authority,
                 &mut Position,
                 &mut Rotation,
                 &mut LinearVelocity,
                 &mut AngularVelocity,
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
                    let Ok((mut network_id2, mut authority2, mut c0, mut c1, mut c2, mut c3)) =
                        query.get_mut(*e)
                    else {
                        continue;
                    };
                    if authority.0 == authority2.as_ref().0 && network_id.1 > network_id2.1 {
                        commands.entity(*e).insert(LocalNet);
                    } else {
                        network_id2.1 = network_id.1;
                    }
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
            |sender: MessageSender<SyncMsg<Physics, (Position, Rotation, LinearVelocity, AngularVelocity)>>,
             query: Query<
                 (&NetworkId, &Authority, &Position, &Rotation, &LinearVelocity, &AngularVelocity),
                 (
                     Or<(
                         Changed<Authority>,
                         Changed<Position>,
                         Changed<Rotation>,
                         Changed<LinearVelocity>,
                         Changed<AngularVelocity>,
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

/*pub type PhysicsSyncPlugin =
    ComponentSyncPlugin<(Position, Rotation, LinearVelocity, AngularVelocity), Physics, Four>;
*/