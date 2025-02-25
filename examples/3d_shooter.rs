use avian3d::{prelude as avian, prelude::*, schedule::PhysicsSchedule};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_matchbox::matchbox_socket::{PeerId, WebRtcSocket};
use bevy_matchbox::MatchboxSocket;
use bevy_tnua::builtins::TnuaBuiltinCrouch;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
    TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::math::{float_consts, AdjustPrecision, AsF32, Float, Quaternion, Vector3};
use bevy_tnua::prelude::*;
use bevy_tnua::{TnuaAnimatingState, TnuaGhostSensor, TnuaToggle};
use bevy_tnua_avian3d::*;
use evnet::{EvnetPlugin, LocalId, NetworkedAppExt, NetworkedEvent, Reliability};
use serde::{Deserialize, Serialize};
use std::ops::Mul;
use tnua_demos_crate::app_setup_options::{AppSetupConfiguration, ScheduleToUse};
use tnua_demos_crate::character_animating_systems::platformer_animating_systems::{
    animate_platformer_character, AnimationState,
};
use tnua_demos_crate::character_control_systems::info_dumpeing_systems::character_control_info_dumping_system;
use tnua_demos_crate::character_control_systems::platformer_control_systems::{
    apply_platformer_controls, CharacterMotionConfigForPlatformerDemo, FallingThroughControlScheme,
    ForwardFromCamera,
};
use tnua_demos_crate::character_control_systems::Dimensionality;
use tnua_demos_crate::level_mechanics::LevelMechanicsPlugin;
use tnua_demos_crate::levels_setup::level_switching::LevelSwitchingPlugin;
use tnua_demos_crate::levels_setup::IsPlayer;
use tnua_demos_crate::ui::component_alterbation::CommandAlteringSelectors;
use tnua_demos_crate::ui::DemoInfoUpdateSystemSet;
use tnua_demos_crate::util::animating::{animation_patcher_system, GltfSceneHandler};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Event)]
pub struct PlayerPosition {
    peer_id: PeerId,
    position: Position,
    rotation: Rotation,
    linear_velocity: LinearVelocity,
}

impl NetworkedEvent for PlayerPosition {
    const RELIABILITY: Reliability = Reliability::UnreliableOrdered;

    fn id(&self) -> PeerId {
        self.peer_id
    }
}

#[derive(Component)]
pub struct Player(PeerId);

fn update_players(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    local_id: Res<LocalId>,
    mut player_positions: EventReader<PlayerPosition>,
    mut players: Query<(&Player, &mut Position, &mut Rotation, &mut LinearVelocity)>,
) {
    let Some(local_id) = local_id.get() else {
        return;
    };
    for player_position in player_positions.read() {
        let mut player_exists = false;
        if player_position.peer_id == local_id {
            continue;
        }
        for (player, mut position, mut rotation, mut linear_velocity) in players.iter_mut() {
            if player.0 != player_position.peer_id {
                continue;
            }
            player_exists = true;
            *position = player_position.position;
            *rotation = player_position.rotation;
            *linear_velocity = player_position.linear_velocity;
        }
        if !player_exists {
            commands.spawn((
                Player(player_position.peer_id),
                RigidBody::Dynamic,
                Transform::default(),
                player_position.position,
                player_position.rotation,
                player_position.linear_velocity,
                Collider::capsule(0.5, 1.0),
                SceneRoot(asset_server.load("player.glb#Scene0")),
            ));
            return;
        }
    }
}

fn send_personal_update(
    mut local_id: Res<LocalId>,
    mut player_positions: EventWriter<PlayerPosition>,
    query: Query<(&Position, &Rotation, &LinearVelocity), With<IsPlayer>>,
) {
    let Some(local_id) = local_id.get() else {
        return;
    };
    let Ok((position, rotation, linear_velocity)) = query.get_single() else {
        return;
    };
    let (position, rotation, linear_velocity) = (*position, *rotation, *linear_velocity);
    player_positions.send(PlayerPosition {
        peer_id: local_id,
        position,
        rotation,
        linear_velocity,
    });
}

fn disconnected_players(
    mut commands: Commands,
    query: Query<(Entity, &Player)>,
    socket: Res<MatchboxSocket>,
) {
    for disconnected in socket.disconnected_peers() {
        for (entity, player) in query.iter() {
            if player.0 == *disconnected {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Event)]
pub struct BallPosition {
    peer_id: PeerId,
    uuid: u32,
    position: Position,
    rotation: Rotation,
    linear_velocity: LinearVelocity,
}

impl NetworkedEvent for BallPosition {
    const RELIABILITY: Reliability = Reliability::UnreliableOrdered;

    fn id(&self) -> PeerId {
        self.peer_id
    }
}

fn shoot_ball(
    local_id: Res<LocalId>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut event_writer: EventWriter<BallPosition>,
    player: Query<(&Position, &Rotation, &ForwardFromCamera), With<IsPlayer>>,
) {
    let Some(local_id) = local_id.get() else {
        return;
    };
    let Ok(player) = player.get_single() else {
        return;
    };
    if mouse_buttons.just_pressed(MouseButton::Left) {
        let position = player.0 .0 + player.2.forward.mul(0.3);
        event_writer.send(BallPosition {
            peer_id: local_id,
            uuid: random_number::random!(),
            position: Position(position),
            rotation: *player.1,
            linear_velocity: LinearVelocity(player.2.forward.mul(20.0)),
        });
    }
}

#[derive(Component)]
pub struct Ball(u32);

#[derive(Component)]
pub struct LocallyControlled;

fn set_updated_ball_pos(
    mut commands: Commands,
    local_id: Res<LocalId>,
    mut event_reader: EventReader<BallPosition>,
    mut balls: Query<(&Ball, &mut Position, &mut Rotation, &mut LinearVelocity)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(local_id) = local_id.get() else {
        return;
    };
    for ball_position in event_reader.read() {
        let mut ball_exists = false;
        for (ball, mut position, mut rotation, mut linear_velocity) in balls.iter_mut() {
            if ball.0 != ball_position.uuid {
                continue;
            }
            ball_exists = true;
            *position = ball_position.position;
            *rotation = ball_position.rotation;
            *linear_velocity = ball_position.linear_velocity;
        }
        if !ball_exists {
            let mut e = commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.1))),
                MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
                RigidBody::Dynamic,
                Transform::default(),
                ball_position.position,
                ball_position.rotation,
                ball_position.linear_velocity,
                Collider::sphere(0.1),
            ));
            if ball_position.peer_id == local_id {
                e.insert(LocallyControlled);
            }
        }
    }
}

fn write_updated_ball_pos(
    local_id: Res<LocalId>,
    mut event_writer: EventWriter<BallPosition>,
    balls: Query<(&Ball, &Position, &Rotation, &LinearVelocity), With<LocallyControlled>>,
) {
    let Some(local_id) = local_id.get() else {
        return;
    };
    for (ball, position, rotation, linear_velocity) in balls.iter() {
        event_writer.send(BallPosition {
            peer_id: local_id,
            uuid: ball.0,
            position: *position,
            rotation: *rotation,
            linear_velocity: *linear_velocity,
        });
    }
}

pub struct MyNetworkingPlugin;
impl Plugin for MyNetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.register_networked_event::<PlayerPosition>();
        app.register_networked_event::<BallPosition>();
        app.add_systems(
            Update,
            (
                update_players,
                send_personal_update,
                disconnected_players,
                shoot_ball,
                set_updated_ball_pos,
                write_updated_ball_pos,
            ),
        );
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        MyNetworkingPlugin,
        EvnetPlugin::with_connection("wss://mb.v-sekai.cloud/3d_shooter_example-room-1"),
    ));

    let app_setup_configuration = AppSetupConfiguration::from_environment();
    app.insert_resource(app_setup_configuration.clone());
    app.add_plugins(PhysicsPlugins::new(PostUpdate));
    app.add_plugins(TnuaAvian3dPlugin::new(Update));
    app.add_plugins(TnuaControllerPlugin::default());
    app.add_plugins(TnuaCrouchEnforcerPlugin::default());

    app.add_systems(
        Update,
        character_control_info_dumping_system.in_set(DemoInfoUpdateSystemSet),
    );
    app.add_plugins(tnua_demos_crate::ui::DemoUi::<
        CharacterMotionConfigForPlatformerDemo,
    >::default());
    app.add_systems(Startup, setup_camera_and_lights);
    app.add_plugins({
        LevelSwitchingPlugin::new(app_setup_configuration.level_to_load.as_ref()).with(
            "Default",
            tnua_demos_crate::levels_setup::for_3d_platformer::setup_level,
        )
    });
    app.add_systems(Startup, setup_player);
    app.add_systems(Update, grab_ungrab_mouse);
    app.add_systems(PostUpdate, {
        let system = apply_camera_controls;
        let system = system.after(avian3d::prelude::PhysicsSet::Sync);
        system.before(bevy::transform::TransformSystem::TransformPropagate)
    });
    app.add_systems(
        Update,
        apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
    );
    app.add_systems(Update, animation_patcher_system);
    app.add_systems(Update, animate_platformer_character);
    app.add_plugins(LevelMechanicsPlugin);
    app.run();
}

fn setup_camera_and_lights(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 16.0, 40.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
    ));

    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 5.0, 5.0)));

    commands.spawn((
        DirectionalLight {
            illuminance: 4000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::default().looking_at(-Vec3::Y, Vec3::Z),
    ));
}
#[derive(PhysicsLayer, Default)]
pub enum LayerNames {
    #[default]
    Default,
    Player,
    FallThrough,
    PhaseThrough,
}
fn setup_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut cmd = commands.spawn(IsPlayer);
    cmd.insert(SceneRoot(asset_server.load("player.glb#Scene0")));
    cmd.insert(GltfSceneHandler {
        names_from: asset_server.load("player.glb"),
    });

    // The character entity must be configured as a dynamic rigid body of the physics backend.
    cmd.insert(avian::RigidBody::Dynamic);
    cmd.insert(avian::Collider::capsule(0.5, 1.0));

    // `TnuaController` is Tnua's main interface with the user code
    cmd.insert(TnuaController::default());

    cmd.insert(CharacterMotionConfigForPlatformerDemo {
        dimensionality: Dimensionality::Dim3,
        speed: 20.0,
        walk: TnuaBuiltinWalk {
            float_height: 2.0,
            max_slope: float_consts::FRAC_PI_4,
            turning_angvel: Float::INFINITY,
            ..Default::default()
        },
        actions_in_air: 1,
        jump: TnuaBuiltinJump {
            height: 4.0,
            ..Default::default()
        },
        crouch: TnuaBuiltinCrouch {
            float_offset: -0.9,
            ..Default::default()
        },
        dash_distance: 10.0,
        dash: Default::default(),
        one_way_platforms_min_proximity: 1.0,
        falling_through: FallingThroughControlScheme::SingleFall,
        knockback: Default::default(),
    });

    cmd.insert(ForwardFromCamera::default());
    cmd.insert(TnuaToggle::default());
    cmd.insert(TnuaAnimatingState::<AnimationState>::default());

    cmd.insert({
        let command_altering_selectors = CommandAlteringSelectors::default()
            .with_combo(
                "Sensor Shape",
                1,
                &[
                    ("no", |mut cmd| {
                        cmd.remove::<TnuaAvian3dSensorShape>();
                    }),
                    ("flat (underfit)", |mut cmd| {
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.49, 0.0)));
                    }),
                    ("flat (exact)", |mut cmd| {
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.5, 0.0)));
                    }),
                    ("flat (overfit)", |mut cmd| {
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.51, 0.0)));
                    }),
                    ("ball (underfit)", |mut cmd| {
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::sphere(0.49)));
                    }),
                    ("ball (exact)", |mut cmd| {
                        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::sphere(0.5)));
                    }),
                ],
            )
            .with_checkbox("Lock Tilt", true, |mut cmd, lock_tilt| {
                if lock_tilt {
                    cmd.insert(avian::LockedAxes::new().lock_rotation_x().lock_rotation_z());
                } else {
                    cmd.insert(avian::LockedAxes::new());
                }
            })
            .with_checkbox(
                "Phase Through Collision Groups",
                true,
                |mut cmd, use_collision_groups| {
                    let player_layers: LayerMask = if use_collision_groups {
                        [LayerNames::Default, LayerNames::Player].into()
                    } else {
                        [
                            LayerNames::Default,
                            LayerNames::Player,
                            LayerNames::PhaseThrough,
                        ]
                        .into()
                    };
                    cmd.insert(CollisionLayers::new(player_layers, player_layers));
                },
            );
        command_altering_selectors
    });

    // `TnuaCrouchEnforcer` can be used to prevent the character from standing up when obstructed.
    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vector3::Y, |cmd| {
        cmd.insert(TnuaAvian3dSensorShape(avian::Collider::cylinder(0.5, 0.0)));
    }));

    // The ghost sensor is used for detecting ghost platforms
    cmd.insert(TnuaGhostSensor::default());
    cmd.insert(TnuaSimpleFallThroughPlatformsHelper::default());
    cmd.insert(TnuaSimpleAirActionsCounter::default());

    cmd.insert((
        //tnua_demos_crate::ui::TrackedEntity("Player".to_owned()),
        //InfoSource::default(),
    ));
}

fn grab_ungrab_mouse(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut primary_window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = primary_window_query.get_single_mut() else {
        return;
    };
    if window.cursor_options.visible {
        if mouse_buttons.just_pressed(MouseButton::Left) {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
    } else if keyboard.just_released(KeyCode::Escape)
    /*|| mouse_buttons.just_pressed(MouseButton::Left)*/
    {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn apply_camera_controls(
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_character_query: Query<(&GlobalTransform, &mut ForwardFromCamera)>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    let mouse_controls_camera = primary_window_query
        .get_single()
        .map_or(false, |w| !w.cursor_options.visible);
    let total_delta = if mouse_controls_camera {
        mouse_motion.read().map(|event| event.delta).sum()
    } else {
        mouse_motion.clear();
        Vec2::ZERO
    };
    let Ok((player_transform, mut forward_from_camera)) = player_character_query.get_single_mut()
    else {
        return;
    };

    let yaw = Quaternion::from_rotation_y(-0.01 * total_delta.x.adjust_precision());
    forward_from_camera.forward = yaw.mul_vec3(forward_from_camera.forward);

    let pitch = 0.005 * total_delta.y.adjust_precision();
    forward_from_camera.pitch_angle = (forward_from_camera.pitch_angle + pitch)
        .clamp(-float_consts::FRAC_PI_2, float_consts::FRAC_PI_2);

    for mut camera in camera_query.iter_mut() {
        camera.translation = player_transform.translation()
            + -5.0 * forward_from_camera.forward.f32()
            + 1.0 * Vec3::Y;
        camera.look_to(forward_from_camera.forward.f32(), Vec3::Y);
        let pitch_axis = camera.left();
        camera.rotate_around(
            player_transform.translation(),
            Quat::from_axis_angle(*pitch_axis, forward_from_camera.pitch_angle.f32()),
        );
    }
}
