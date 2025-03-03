use avian3d::prelude::*;
use bevy::prelude::*;
use evnet::component_sync_layer::{DespawnOnDisconnect, LocalNet, NetworkEntityMapper, NetworkId};
use evnet::event_layer::{AppExt2, NetworkEventReader, NetworkEventWriter};
use evnet::message_layer::NetworkMessage;
use evnet::physics_layer::{PhysicsSyncPlugin};
use evnet::{
    Me, NetworkedCommandExt, NetworkingPlugins, PeerConnected, Reliability,
    connected,
};
use serde::{Deserialize, Serialize};

fn main() {
    App::new() // Enable physics
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
            NetworkingPlugins,
            PhysicsSyncPlugin::default(),
        ))
        .add_networked_event::<SpawnCube>()
        .add_networked_event::<KillCube>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_spawn_cube,
                cube_move,
                changed,
                sync_colors,
                kill_cube_out_of_bounds,
                actually_kill_cube,
                peer_connected,
            )
                .chain()
                .run_if(connected),
        )
        .run();
}

pub fn peer_connected(mut event_reader: EventReader<PeerConnected>) {
    for _ in event_reader.read() {
        println!("Peer connected");
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct KillCube(pub NetworkId);

impl NetworkMessage for KillCube {
    const RELIABILITY: Reliability = Reliability::Reliable;
}

fn kill_cube_out_of_bounds(
    mut event_writer: NetworkEventWriter<KillCube>,
    cubes: Query<(&NetworkId, &Transform), (With<Cube>, With<LocalNet>)>,
) {
    cubes.iter().for_each(|(sync_net, t)| {
        if t.translation.y <= -100.0 {
            event_writer.send(KillCube(*sync_net));
        }
    });
}

fn actually_kill_cube(
    mut event_reader: NetworkEventReader<KillCube>,
    mut commands: Commands,
    entity_mapper: Res<NetworkEntityMapper>,
) {
    for (_peer, msg) in event_reader.read() {
        let Some(e) = entity_mapper.get(&msg.0) else {
            continue;
        };
        let Some(e) = commands.get_entity(*e) else {
            continue;
        };
        e.despawn_recursive();
        //println!("I actually killed");
    }
}

fn changed(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    remote: Query<Entity, (With<Cube>, Without<LocalNet>)>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        for e in remote.iter() {
            commands.entity(e).insert(LocalNet);
            break;
        }
    }
}

fn sync_colors(
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<LocalNet>>,
    query2: Query<&MeshMaterial3d<StandardMaterial>, Without<LocalNet>>,
) {
    for q in query.iter() {
        materials.get_mut(q).unwrap().base_color = Color::BLACK;
    }
    for q in query2.iter() {
        materials.get_mut(q).unwrap().base_color = Color::WHITE;
    }
}

fn cube_move(
    me: Me,
    keys: Res<ButtonInput<KeyCode>>,
    mut event_writer: NetworkEventWriter<SpawnCube>,
    mut cubes: Query<&mut LinearVelocity, With<LocalNet>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        event_writer.send(SpawnCube::new(&me));
    }
    const AMOUNT: f32 = 0.2;
    for mut cube in cubes.iter_mut() {
        if keys.pressed(KeyCode::KeyA) {
            cube.x -= AMOUNT;
        }
        if keys.pressed(KeyCode::KeyD) {
            cube.x += AMOUNT;
        }
        if keys.pressed(KeyCode::KeyW) {
            cube.y += AMOUNT;
        }
        if keys.pressed(KeyCode::KeyS) {
            cube.y -= AMOUNT;
        }
    }
}

#[derive(Component)]
pub struct Cube;

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct SpawnCube(NetworkId);
impl SpawnCube {
    pub fn new(me: &Me) -> Self {
        Self(NetworkId::new(me))
    }
}
impl NetworkMessage for SpawnCube {
    const RELIABILITY: Reliability = Reliability::Reliable;
}
fn handle_spawn_cube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_reader: NetworkEventReader<SpawnCube>,
    me: Me,
) {
    for (peer, spawn_cube) in event_reader.read() {
        let physics_sync = spawn_cube.0;
        // Dynamic physics object with a collision shape and initial angular velocity
        let mut entity = commands.spawn((
            physics_sync,
            RigidBody::Dynamic,
            Collider::cuboid(0.1, 0.1, 0.1),
            AngularVelocity(Vec3::new(2.5, 3.5, 1.5)),
            Mesh3d(meshes.add(Cuboid::from_length(0.1))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 4.0, 0.0),
            Cube,
            DespawnOnDisconnect(*peer),
            TransformInterpolation,
        ));
        if peer == me.get() {
            entity.insert(LocalNet);
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.connect("wss://mb.v-sekai.cloud/my-room-2");
    // Static physics object with a collision shape
    commands.spawn((
        RigidBody::Static,
        Collider::cylinder(6.0, 0.1),
        Mesh3d(meshes.add(Cylinder::new(6.0, 0.1))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Dir3::Y),
    ));
}
