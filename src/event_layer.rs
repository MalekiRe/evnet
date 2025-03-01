use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use crate::{Me, Peer};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;

pub trait AppExt2 {
    fn add_networked_event<
        T: Clone + 'static + NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    >(
        &mut self,
    ) -> &mut Self;
}
#[derive(Event)]
pub struct NetworkEvent<T: Clone + Serialize + for<'de> Deserialize<'de> + 'static>(
    pub Peer,
    pub T,
);
impl<T: Clone + Serialize + for<'de> Deserialize<'de> + 'static> NetworkEvent<T> {
    pub fn new(peer: Peer, inner: T) -> Self {
        Self(peer, inner)
    }
}

impl AppExt2 for App {
    fn add_networked_event<
        T: Clone + 'static + NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_network_message(
            |rx: MessageReceiver<T>, mut event_writer: EventWriter<NetworkEvent<T>>| {
                for (e, peer) in rx.try_iter() {
                    event_writer.send(NetworkEvent::new(peer, e));
                }
            },
        );
        self.add_systems(
            Update,
            (|mut event_reader: EventReader<NetworkEvent<T>>,
              me: Res<Me>,
              mut message_sender: MessageSender<T>| {
                for NetworkEvent(peer, inner) in event_reader.read() {
                    if peer == me.0 {
                        message_sender
                            .send((inner.clone(), SendType::AllButSelf))
                            .unwrap();
                    }
                }
            })
            .run_if(resource_exists::<Me>)
            .after(crate::message_layer::route_messages),
        );
        self.add_event::<NetworkEvent<T>>();
        self
    }
}
