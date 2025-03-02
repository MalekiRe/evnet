use std::time::{Duration, Instant};
use bevy::prelude::{Reflect, Resource};
use random_number::rand::{thread_rng, Rng};
use crate::Peer;
use crate::ready_buffer::ReadyBuffer;

/// Contains configuration required to initialize a LinkConditioner
#[derive(Clone, Debug, Reflect)]
pub struct LinkConditionerConfig {
    /// Delay to receive incoming messages in milliseconds (half the RTT)
    pub incoming_latency: Duration,
    /// The maximum additional random latency to delay received incoming
    /// messages in milliseconds. This may be added OR subtracted from the
    /// latency determined in the `incoming_latency` property above
    pub incoming_jitter: Duration,
    /// The % chance that an incoming packet will be dropped.
    /// Represented as a value between 0 and 1
    pub incoming_loss: f32,
}

#[derive(Resource)]
pub struct LinkConditioner<P> {
    phantom_data: std::marker::PhantomData<P>,
    config: LinkConditionerConfig,
    pub time_queue: ReadyBuffer<Instant, (Vec<u8>, Peer)>,
    last_packet: Option<(Vec<u8>, Peer)>,
}

impl<P> LinkConditioner<P> {
    pub fn new(config: LinkConditionerConfig) -> Self {
        LinkConditioner {
            phantom_data: Default::default(),
            config,
            time_queue: ReadyBuffer::new(),
            last_packet: None,
        }
    }

    /// Add latency/jitter/loss to a packet
    pub fn condition_packet(&mut self, packet: Vec<u8>, peer: Peer) {
        let mut rng = thread_rng();
        if rng.gen_range(0.0..1.0) <= self.config.incoming_loss {
            return;
        }
        let mut latency: i32 = self.config.incoming_latency.as_millis() as i32;
        // TODO: how can i use the virtual time here?
        let mut packet_timestamp = Instant::now();
        if self.config.incoming_jitter > Duration::default() {
            let jitter: i32 = self.config.incoming_jitter.as_millis() as i32;
            latency += rng.gen_range(-jitter..jitter);
        }
        if latency > 0 {
            packet_timestamp += Duration::from_millis(latency as u64);
        }
        self.time_queue.push(packet_timestamp, (packet, peer));
    }

    /// Check if a packet is ready to be returned
    pub fn pop_packet(&mut self) -> Option<(Vec<u8>, Peer)> {
        self.time_queue
            .pop_item(&Instant::now())
            .map(|(_, packet)| packet)
    }
}
