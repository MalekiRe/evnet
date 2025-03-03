use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use crate::{Peer};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub trait AppExt2 {
    fn add_networked_event<
        T: Clone + 'static + NetworkMessage + Serialize + for<'de> Deserialize<'de>,
    >(
        &mut self,
    ) -> &mut Self;
}

#[derive(SystemParam)]
pub struct NetworkEventReader<
    'w,
    's,
    E: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
> {
    event_reader: EventReader<'w, 's, NetworkEvent<E>>,
}

impl<E: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static>
    NetworkEventReader<'_, '_, E>
{
    pub fn read(&mut self) -> impl Iterator<Item = (&Peer, &E)> + '_ {
        self.event_reader
            .read()
            .map(|NetworkEvent(peer, event)| (peer, event))
    }
}

#[derive(SystemParam)]
pub struct NetworkEventWriter<
    'w,
    E: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
> {
    message_sender: MessageSender<'w, E>,
}
impl<E: Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static>
    NetworkEventWriter<'_, E>
{
    pub fn send(&mut self, e: E) {
        self.send_to(e, SendType::All);
    }
    pub fn send_to(&mut self, e: E, send_type: SendType) {
        self.message_sender.send((e, send_type)).unwrap();
    }
}

#[derive(Event)]
struct NetworkEvent<E: Clone + Serialize + for<'de> Deserialize<'de> + 'static>(Peer, E);

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
        self.add_event::<NetworkEvent<T>>();
        self
    }
}
