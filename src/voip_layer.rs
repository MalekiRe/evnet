use crate::event_layer::{AppExt2, NetworkEventReader, NetworkEventWriter};
use crate::message_layer::{AppExt, MessageReceiver, MessageSender, NetworkMessage, SendType};
use crate::{Me, Peer, PeerConnected, Reliability};
use bevy::app::App;
use bevy::prelude::{Commands, Component, Entity, EventReader, In, IntoSystemConfigs, Local, Mut, NonSendMut, Plugin, PostUpdate, Query, Res, ResMut, Resource, Startup, Update, Without, World};
use bevy_mod_audio::ModAudioPlugins;
use bevy_mod_audio::audio_output::AudioOutput;
use bevy_mod_audio::microphone::{MicrophoneAudio, MicrophoneConfig};
use bevy_mod_audio::spatial_audio::SpatialAudioSink;
use opus::{Application, Channels, Decoder, Encoder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct VoipPlugin;
impl Plugin for VoipPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ModAudioPlugins);
        app.add_network_event::<MicrophoneConfigInfo>();
        app.add_network_message(
            |rx: MessageReceiver<VoipMessage>,
             mut decoder: NonSendMut<MicrophoneDecoder>,
             query: Query<(&Peer, &MicrophoneConfigInfo, &SpatialAudioSink)>| {
                let mut a = [0.0; 2880];
                let mut b = [0.0; 2880 * 2];
                for (msg, peer) in rx.try_iter() {
                    for (other_peer, info, sink) in query.iter() {
                        if *other_peer == peer {
                            println!("got voice message");
                            let decoder = decoder.0.get_mut(&info.as_tuple()).unwrap();
                            let output = match info.channels {
                                1 => {
                                    a.as_mut_slice()
                                }
                                2 => {
                                    b.as_mut_slice()
                                }
                                _ => unimplemented!(),
                            };
                            decoder.decode_float(&msg.data, output, false).unwrap();
                            sink.append(rodio::buffer::SamplesBuffer::new(
                                info.channels,
                                info.sample_rate,
                                output,
                            ));
                        }
                    }
                }
            },
        );
        app.add_systems(PostUpdate, add_config_info);
        app.add_systems(Update, send_config_info);
        app.add_systems(Startup, setup_encoder_decoder.after(bevy_mod_audio::microphone::create_microphone));
        app.add_systems(Update, send_voice_message);
    }
}

pub fn send_voice_message(
    mut encoder: NonSendMut<MicrophoneEncoder>,
    ev: MessageSender<VoipMessage>,
    microphone: ResMut<MicrophoneAudio>,
    mut stored_audio: Local<Vec<f32>>,
) {
    for mut data in microphone.try_iter() {
        stored_audio.append(&mut data);
    }
    if stored_audio.len() < 2880 {
        return;
    }
    while stored_audio.len() > 2880 {
        let data = encoder
            .0
            .encode_vec_float(stored_audio.drain(0..2880).as_ref(), 2880)
            .unwrap();
        ev.send((VoipMessage { data }, SendType::AllButSelf))
            .unwrap()
    }
}

pub fn setup_encoder_decoder(world: &mut World) {
    world.resource_scope(|world, microphone: Mut<MicrophoneAudio>| {
        let encoder = Encoder::new(
            microphone.config.sample_rate,
            match microphone.config.channels {
                1 => Channels::Mono,
                2 => Channels::Stereo,
                _ => unimplemented!(),
            },
            Application::Voip,
        )
        .unwrap();
        world.insert_non_send_resource(MicrophoneEncoder(encoder));
        world.insert_non_send_resource(MicrophoneDecoder(HashMap::default()));
    });
}

#[derive(Resource)]
pub struct MicrophoneEncoder(pub Encoder);
#[derive(Resource)]
pub struct MicrophoneDecoder(HashMap<(u32, Channels), Decoder>);
unsafe impl Sync for MicrophoneEncoder {}
unsafe impl Sync for MicrophoneDecoder {}

#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct MicrophoneConfigInfo {
    pub channels: u16,
    pub sample_rate: u32,
}
impl MicrophoneConfigInfo {
    pub fn as_tuple(&self) -> (u32, Channels) {
        (
            self.sample_rate,
            match self.channels {
                1 => Channels::Mono,
                2 => Channels::Stereo,
                _ => unimplemented!(),
            },
        )
    }
}

impl NetworkMessage for MicrophoneConfigInfo {
    const RELIABILITY: Reliability = Reliability::Reliable;
}

pub fn send_config_info(
    microphone: Res<MicrophoneAudio>,
    mut peer_connected: EventReader<PeerConnected>,
    mut ev: NetworkEventWriter<MicrophoneConfigInfo>,
) {
    for peer_connected in peer_connected.read() {
        ev.send_to(
            MicrophoneConfigInfo {
                channels: microphone.config.channels,
                sample_rate: microphone.config.sample_rate,
            },
            SendType::One(peer_connected.0),
        );
    }
}

pub fn add_config_info(
    mut commands: Commands,
    query: Query<(Entity, &Peer), Without<MicrophoneConfigInfo>>,
    mut ev: NetworkEventReader<MicrophoneConfigInfo>,
    awa: Res<AudioOutput>,
    mut decoder: NonSendMut<MicrophoneDecoder>,
) {
    for (peer, microphone_config_info) in ev.read() {
        println!("A");
        for (entity, peer_without_config) in query.iter() {
            println!("B");
            if peer_without_config == peer {
                println!("C");
                commands
                    .entity(entity)
                    .insert(microphone_config_info.clone());
                commands.entity(entity).insert(awa.new_sink().unwrap());
                if !decoder.0.contains_key(&microphone_config_info.as_tuple()) {
                    decoder.0.insert(
                        microphone_config_info.as_tuple(),
                        Decoder::new(
                            microphone_config_info.sample_rate,
                            microphone_config_info.as_tuple().1,
                        )
                        .unwrap(),
                    );
                }
            }
        }
    }
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct VoipMessage {
    data: Vec<u8>,
}
impl NetworkMessage for VoipMessage {
    const RELIABILITY: Reliability = Reliability::UnreliableOrdered;
}
