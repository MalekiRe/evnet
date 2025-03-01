use bevy::prelude::*;
use evnet::event_layer::{AppExt2, NetworkEvent};
use evnet::message_layer::NetworkMessage;
use evnet::{Me, NetworkedCommandExt, NetworkingPlugins, Reliability};
use serde::{Deserialize, Serialize};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, NetworkingPlugins))
        .add_systems(Startup, setup)
        .add_systems(Update, (update, update2).run_if(resource_exists::<Me>))
        .add_networked_event::<MyMsg>()
        .run();
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MyMsg {
    owo: u32,
}
impl NetworkMessage for MyMsg {
    const RELIABILITY: Reliability = Reliability::Reliable;
}

fn setup(mut commands: Commands) {
    commands.connect("wss://mb.v-sekai.cloud/my-room-1");
}

fn update(mut event_writer: EventWriter<NetworkEvent<MyMsg>>, me: Res<Me>) {
    event_writer.send(NetworkEvent(me.get(), MyMsg { owo: 42 }));
}

fn update2(mut event_reader: EventReader<NetworkEvent<MyMsg>>, me: Res<Me>) {
    for NetworkEvent(peer, my_msg) in event_reader.read() {
        if peer == me.get() {
            continue;
        }
        println!("{my_msg:?}, {peer:?}");
    }
}
