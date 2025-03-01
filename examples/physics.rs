use avian3d::prelude::*;
use bevy::asset::AssetContainer;
use bevy::prelude::*;
use evnet::component_sync_layer::{LocalNet, SyncNet};
use evnet::event_layer::{AppExt2, NetworkEvent};
use evnet::message_layer::NetworkMessage;
use evnet::physics_layer::{Physics, PhysicsSyncPlugin};
use evnet::{Me, NetworkedCommandExt, NetworkingPlugins, Reliability};
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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_spawn_cube, cube_move, changed, sync_colors)
                .chain()
                .run_if(resource_exists::<Me>),
        )
        .run();
}

fn changed(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    remote: Query<Entity, (With<Cube>, Without<LocalNet<Physics>>)>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        for e in remote.iter() {
            commands.entity(e).insert(LocalNet::<Physics>::default());
            break;
        }
    }
}

fn sync_colors(
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<LocalNet<Physics>>>,
    query2: Query<&MeshMaterial3d<StandardMaterial>, Without<LocalNet<Physics>>>,
) {
    for q in query.iter() {
        materials.get_mut(q).unwrap().base_color = Color::BLACK;
    }
    for q in query2.iter() {
        materials.get_mut(q).unwrap().base_color = Color::WHITE;
    }
}

fn cube_move(
    me: Res<Me>,
    keys: Res<ButtonInput<KeyCode>>,
    mut event_writer: EventWriter<NetworkEvent<SpawnCube>>,
    mut cubes: Query<&mut LinearVelocity, With<LocalNet<Physics>>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        event_writer.send(NetworkEvent(me.get(), SpawnCube::new()));
    }
    const AMOUNT: f32 = 1.0;
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
pub struct SpawnCube(SyncNet<Physics>);
impl SpawnCube {
    pub fn new() -> Self {
        Self(SyncNet::new())
    }
}
impl NetworkMessage for SpawnCube {
    const RELIABILITY: Reliability = Reliability::Reliable;
}
fn handle_spawn_cube(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_reader: EventReader<NetworkEvent<SpawnCube>>,
    me: Res<Me>,
) {
    for NetworkEvent(peer, spawn_cube) in event_reader.read() {
        let physics_sync = spawn_cube.0;
        // Dynamic physics object with a collision shape and initial angular velocity
        let mut entity = commands.spawn((
            physics_sync,
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 1.0),
            AngularVelocity(Vec3::new(2.5, 3.5, 1.5)),
            Mesh3d(meshes.add(Cuboid::from_length(1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            Transform::from_xyz(0.0, 4.0, 0.0),
            Cube,
        ));
        if peer == me.get() {
            entity.insert(LocalNet::<Physics>::default());
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
        Collider::cylinder(4.0, 0.1),
        Mesh3d(meshes.add(Cylinder::new(4.0, 0.1))),
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
