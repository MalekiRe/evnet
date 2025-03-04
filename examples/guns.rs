use avian3d::PhysicsPlugins;
use avian3d::math::Vector;
use avian3d::prelude::{
    CoefficientCombine, Collider, ExternalForce, ExternalImpulse, Friction, LinearVelocity,
    LockedAxes, Mass, Position, RigidBody,
};
use bevy::prelude::*;
use evnet::component_sync_layer::{DespawnOnDisconnect, LocalNet, NetworkEntityMapper, NetworkId};
use evnet::event_layer::{AppExt2, NetworkEventReader, NetworkEventWriter};
use evnet::message_layer::SendType;
use evnet::physics_layer::PhysicsSyncPlugin;
use evnet::{Me, NetworkedCommandExt, NetworkingPlugins, Peer, PeerConnected, connected, just_connected, first_peer_connected};
use evnet_macros::NetworkMessage;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, MulAssign};

pub const FLOOR_WIDTH: f32 = 100.0;
pub const FLOOR_HEIGHT: f32 = 1.0;

pub const BLOCK_WIDTH: f32 = 1.0;
pub const BLOCK_HEIGHT: f32 = 1.0;

pub const CHARACTER_CAPSULE_RADIUS: f32 = 0.5;
pub const CHARACTER_CAPSULE_HEIGHT: f32 = 0.5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NetworkingPlugins)
        .add_plugins(PhysicsSyncPlugin::default())
        .add_plugins(PhysicsPlugins::default())
        .add_network_event::<SpawnPlayer>()
        .add_network_event::<SpawnBullet>()
        .add_network_event::<GibAllData>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                on_spawn_player,
                on_press,
                spawn_bullet,
                when_gib_all_data_received,
                respawn,
            )
                .run_if(connected),
        )
        //.add_systems(Update, on_self_connect.run_if(just_connected))
        .add_systems(Update, (try_get_peer, on_self_connect).run_if(first_peer_connected))
        .run();
}

#[derive(Bundle)]
pub(crate) struct CharacterPhysicsBundle {
    collider: Collider,
    rigid_body: RigidBody,
    external_force: ExternalForce,
    external_impulse: ExternalImpulse,
    lock_axes: LockedAxes,
    friction: Friction,
    mesh: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
}

fn respawn(mut query: Query<&mut Transform, With<Player>>) {
    for mut t in query.iter_mut() {
        if t.translation.y <= -30.0 {
            t.translation = Vec3::new(0.0, 10.0, 0.0);
        }
    }
}

impl CharacterPhysicsBundle {
    fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            collider: Collider::capsule(CHARACTER_CAPSULE_RADIUS, CHARACTER_CAPSULE_HEIGHT),
            rigid_body: RigidBody::Dynamic,
            external_force: ExternalForce::ZERO.with_persistence(false),
            external_impulse: ExternalImpulse::ZERO.with_persistence(false),
            lock_axes: LockedAxes::default()
                .lock_rotation_x()
                .lock_rotation_y()
                .lock_rotation_z(),
            friction: Friction::new(0.5).with_combine_rule(CoefficientCombine::Min),
            mesh: Mesh3d(meshes.add(Capsule3d::new(
                CHARACTER_CAPSULE_RADIUS,
                CHARACTER_CAPSULE_HEIGHT,
            ))),
            material: MeshMaterial3d(materials.add(Color::srgb_u8(230, 30, 30))),
        }
    }
}

#[derive(NetworkMessage, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct SpawnPlayer(NetworkId, Vec3);

#[derive(NetworkMessage, Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct SpawnBullet(NetworkId, Vec3, Vec3);

#[derive(Component)]
pub struct Player(Peer);

#[derive(NetworkMessage, Deserialize, Serialize, Clone, Debug)]
pub struct GibAllData;

#[derive(Component)]
pub struct Bullet;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.connect("wss://mb.v-sekai.cloud/my-room-3");
    commands.spawn((
        Collider::cuboid(FLOOR_WIDTH, FLOOR_HEIGHT, FLOOR_WIDTH),
        RigidBody::Static,
        Mesh3d(meshes.add(Cuboid::new(FLOOR_WIDTH, FLOOR_HEIGHT, FLOOR_WIDTH))),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.5, 0.6))),
        Transform::from_xyz(0.0, -3.0, 0.0),
    ));

    commands.spawn((
        Collider::cuboid(0.5, 0.5, 0.5),
        RigidBody::Static,
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 0.8, 0.8))),
        Transform::from_xyz(1.0, -2.0, 0.0),
    ));

    commands.spawn((
        Collider::cuboid(0.5, 0.5, 0.5),
        RigidBody::Static,
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.0, 0.8, 0.8))),
        Transform::from_xyz(1.0, -2.0, -3.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        SpotLight {
            intensity: 100_000.0,
            /*color: LIME.into(),*/
            shadows_enabled: false,
            inner_angle: 1.0,
            outer_angle: 3.0,
            ..default()
        },
        Transform::from_xyz(-1.0, 2.0, 0.0).looking_at(Vec3::new(-1.0, 0.0, 0.0), Vec3::Z),
    ));
}

fn on_spawn_player(
    mut commands: Commands,
    mut ev: NetworkEventReader<SpawnPlayer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    network_entity_mapper: Res<NetworkEntityMapper>,
    me: Me,
) {
    for (peer, spawn_player) in ev.read() {
        if network_entity_mapper.contains_key(&spawn_player.0) {
            continue;
        }
        let mut e = commands.spawn((
            CharacterPhysicsBundle::new(&mut meshes, &mut materials),
            spawn_player.0,
            Transform::from_translation(spawn_player.1),
            Player(*peer),
            DespawnOnDisconnect(*peer),
        ));
        if me == peer {
            e.insert(LocalNet);
            e.insert(MeshMaterial3d(materials.add(Color::srgb(0.0, 0.0, 1.0))));
        }
    }
}

fn on_press(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<
        (&mut Position, &mut LinearVelocity, &mut Transform),
        (With<Player>, With<LocalNet>, Without<Camera3d>),
    >,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut ev: NetworkEventWriter<SpawnBullet>,
    me: Me,
) {
    let Ok((mut position, mut velocity, mut transform)) = query.get_single_mut() else {
        return;
    };

    const AMOUNT: f32 = 0.1;

    if keys.pressed(KeyCode::KeyA) {
        transform.rotation.mul_assign(Quat::from_axis_angle(Vec3::Y, 0.1));
    }
    if keys.pressed(KeyCode::KeyD) {
        transform.rotation.mul_assign(Quat::from_axis_angle(Vec3::Y, -0.1));
    }
    let forward = transform.rotation.mul_vec3(Vec3::new(0.0, 0.0, -AMOUNT));

    if keys.just_pressed(KeyCode::Space) {
        ev.send(SpawnBullet(
            NetworkId::new(&me),
            transform.translation.add(forward.mul(5.0)),
            forward.mul(300.0)
        ));
    }

    if keys.pressed(KeyCode::KeyW) {
        //position.z += forward.z;
        //position.x += forward.x;
        velocity.z += forward.z;
        velocity.x += forward.x;
    }
    if keys.pressed(KeyCode::KeyS) {
        //position.z -= forward.z;
        //position.x -= forward.x;
        velocity.z -= forward.z;
        velocity.x -= forward.x;
    }

    if keys.pressed(KeyCode::KeyE) {
        velocity.y += AMOUNT * 2.0;
    }
    if keys.pressed(KeyCode::KeyR) {
        velocity.y -= AMOUNT * 2.0;
    }

    if let Ok(mut cam_transform) = camera.get_single_mut() {
        // Position the camera at a fixed offset behind and above the player
        // Higher Y value and further back Z value for 45-degree angle
        let behind_offset = 30.0; // Distance behind player
        let height_offset = 10.0; // Height above player

        // Get the backward direction from the player's rotation (opposite of forward)
        let backward_dir = transform.rotation.mul_vec3(Vec3::new(0.0, 0.0, 1.0)).normalize();

        // Calculate camera position
        let camera_pos = transform.translation +
            (backward_dir * behind_offset) +
            Vec3::new(0.0, height_offset, 0.0);

        cam_transform.translation = camera_pos;

        // Create a rotation that looks from the camera position to the player
        // This creates a stable 45-degree downward angle
        let look_dir = (transform.translation - camera_pos).normalize();

        // Use a stable up vector to prevent flipping
        let up = Vec3::Y;

        // Create look-at rotation that won't flip
        let forward = -look_dir; // Camera looks in the negative Z direction
        let right = up.cross(forward).normalize();
        let corrected_up = forward.cross(right).normalize();

        // Construct a stable rotation matrix
        let rotation_mat = Mat3::from_cols(right, corrected_up, forward);
        cam_transform.rotation = Quat::from_mat3(&rotation_mat);
    }
}

fn on_self_connect(mut ev: NetworkEventWriter<SpawnPlayer>, me: Me) {
    ev.send(SpawnPlayer(NetworkId::new(&me), Vec3::default()));
}

fn try_get_peer(
    mut local: Local<bool>,
    mut event_reader: EventReader<PeerConnected>,
    mut ev: NetworkEventWriter<GibAllData>,
) {
    if *local {
        return;
    }
    for peer_connected in event_reader.read() {
        ev.send_to(GibAllData, SendType::One(peer_connected.get()));
        *local = true;
        return;
    }
}

fn when_gib_all_data_received(
    mut ev: NetworkEventReader<GibAllData>,
    mut bullet_ev: NetworkEventWriter<SpawnBullet>,
    mut player_ev: NetworkEventWriter<SpawnPlayer>,
    bullets: Query<(&NetworkId, &Transform), With<Bullet>>,
    players: Query<(&NetworkId, &Transform), With<Player>>,
) {
    for (peer, _) in ev.read() {
        println!("got gib all data");
        for (network_id, transform) in bullets.iter() {
            bullet_ev.send_to(
                SpawnBullet(*network_id, transform.translation, Vec3::ZERO),
                SendType::One(*peer),
            );
        }
        for (player, transform) in players.iter() {
            println!("sending spawn player");
            player_ev.send_to(SpawnPlayer(*player, transform.translation), SendType::One(*peer));
        }
    }
}

fn spawn_bullet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ev: NetworkEventReader<SpawnBullet>,
    me: Me,
) {
    for (peer, SpawnBullet(network_id, position, velocity)) in ev.read() {
        let mut e = commands.spawn((
            RigidBody::Dynamic,
            Collider::sphere(0.1),
            Mesh3d(meshes.add(Sphere::new(0.1))),
            MeshMaterial3d(materials.add(Color::srgb_u8(0, 200, 20))),
            *network_id,
            Mass(30.0),
            LinearVelocity(*velocity),
            Transform::from_translation(*position),
            Bullet,
        ));
        if me == peer {
            e.insert(LocalNet);
        }
    }
}
