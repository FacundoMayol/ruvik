use std::f32::consts::PI;

use super::*;

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings},
    input::mouse::MouseMotion,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

#[derive(Component)]
struct Cube;

#[derive(Debug, Clone, Copy)]
enum CubeAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy)]
enum CubeFace {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct PendingDrag {
    viewport_origin: Vec2,
    axis_0: CubeAxis,
    index_0: u32,
    viewport_dir_0: Vec2,
    axis_1: CubeAxis,
    index_1: u32,
    viewport_dir_1: Vec2,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct ActiveDrag {
    axis: CubeAxis,
    viewport_origin: Vec2,
    viewport_dir: Vec2,
    current_angle: f32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct ActiveCubeRotation {
    axis: CubeAxis,
    current_angle: f32,
    target_rotations: u32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct BeingDragged {
    prev_rotation: Quat,
}

#[derive(Component)]
struct Cubie {
    position: (u32, u32, u32), // (0, 0, 0) is left-bottom-back, (2, 2, 2) is right-top-front
}

const CLEAR_COLOR: Color = Color::srgb(0.40, 0.36, 0.23);

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Game), game_setup)
        .add_systems(
            Update,
            (
                cube_rotation_system,
                (
                    cubie_drag_init_system,
                    cubie_drag_pending_system,
                    cubie_drag_system,
                    cubie_rotation_system,
                )
                    .chain(),
            )
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(OnExit(GameState::Game), game_cleanup);
}

fn cube_rotation_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_events: MessageReader<MouseMotion>,
    mut cube_transform: Single<&mut Transform, With<Cube>>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }

    let mut delta = Vec2::ZERO;
    for event in motion_events.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    let yaw = Quat::from_rotation_y(delta.x * 0.01);
    let pitch = Quat::from_rotation_x(delta.y * 0.01);

    cube_transform.rotate(yaw);
    cube_transform.rotate(pitch);
}

fn cubie_drag_init_system(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    cube: Single<
        (Entity, &GlobalTransform),
        (
            With<Cube>,
            Without<ActiveDrag>,
            Without<PendingDrag>,
            Without<ActiveCubeRotation>,
        ),
    >,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let (camera, global_transform) = camera.into_inner();
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    let Ok(ray) = camera.viewport_to_world(global_transform, cursor_position) else {
        return;
    };

    let inv = cube.1.affine().inverse();

    let local_origin = inv.transform_point3(ray.origin);
    let local_dir = inv.transform_vector3(ray.direction.as_vec3()).normalize();

    let local_ray = Ray3d {
        origin: local_origin,
        direction: Dir3::new(local_dir).expect("Direction should be normalized"),
    };

    let min = Vec3::splat(-0.5);
    let max = Vec3::splat(0.5);

    let inv_dir = 1.0 / local_ray.direction.as_vec3();

    let t1 = (min - local_ray.origin) * inv_dir;
    let t2 = (max - local_ray.origin) * inv_dir;

    let t_min = t1.min(t2);
    let t_max = t1.max(t2);

    let t_enter = t_min.max_element();
    let t_exit = t_max.min_element();

    if t_enter > t_exit || t_exit <= 0.0 {
        return;
    }

    let hit = local_ray.get_point(t_enter);

    const EPS: f32 = 1e-4;

    let hit_face = if (hit.x - 0.5).abs() < EPS {
        CubeFace::PosX
    } else if (hit.x + 0.5).abs() < EPS {
        CubeFace::NegX
    } else if (hit.y - 0.5).abs() < EPS {
        CubeFace::PosY
    } else if (hit.y + 0.5).abs() < EPS {
        CubeFace::NegY
    } else if (hit.z - 0.5).abs() < EPS {
        CubeFace::PosZ
    } else {
        CubeFace::NegZ
    };

    let hit_up_direction = match hit_face {
        CubeFace::PosX => Vec3::Y,
        CubeFace::NegX => -Vec3::Y,
        CubeFace::PosY => Vec3::Z,
        CubeFace::NegY => -Vec3::Z,
        CubeFace::PosZ => -Vec3::Y,
        CubeFace::NegZ => Vec3::Y,
    };

    let hit_right_direction = match hit_face {
        CubeFace::PosX => -Vec3::Z,
        CubeFace::NegX => Vec3::Z,
        CubeFace::PosY => -Vec3::X,
        CubeFace::NegY => Vec3::X,
        CubeFace::PosZ => Vec3::X,
        CubeFace::NegZ => -Vec3::X,
    };

    let viewport_origin = cursor_position;

    let (viewport_dir_0, viewport_dir_1) = {
        let a_local = hit;
        let b_local = hit + hit_up_direction * 0.1;
        let c_local = hit + hit_right_direction * 0.1;
        let a_world = cube.1.transform_point(a_local);
        let b_world = cube.1.transform_point(b_local);
        let c_world = cube.1.transform_point(c_local);
        let a_viewport = match camera.world_to_viewport(global_transform, a_world) {
            Ok(pos) => pos,
            Err(_) => {
                return;
            }
        };
        let b_viewport = match camera.world_to_viewport(global_transform, b_world) {
            Ok(pos) => pos,
            Err(_) => {
                return;
            }
        };
        let c_viewport = match camera.world_to_viewport(global_transform, c_world) {
            Ok(pos) => pos,
            Err(_) => {
                return;
            }
        };
        (
            (b_viewport - a_viewport).normalize_or_zero(),
            (c_viewport - a_viewport).normalize_or_zero(),
        )
    };

    if viewport_dir_0 == Vec2::ZERO || viewport_dir_1 == Vec2::ZERO {
        return;
    }

    let hit_face_uv = match hit_face {
        CubeFace::PosX | CubeFace::NegX => Vec2::new(hit.z, hit.y), // (-0.5, -0.5) is bottom-right facing +X
        CubeFace::PosY | CubeFace::NegY => Vec2::new(hit.x, hit.z), // (-0.5, -0.5) is back-left facing +Y
        CubeFace::PosZ | CubeFace::NegZ => Vec2::new(hit.x, hit.y), // (-0.5, -0.5) is bottom-left facing +Z
    } + Vec2::splat(0.5); // Map from [-0.5, 0.5] to [0, 1]

    let (hit_face_cell_u, hit_face_cell_v) = (
        (hit_face_uv.x * 3.0).floor() as u32,
        (hit_face_uv.y * 3.0).floor() as u32,
    );

    let axis_0 = match hit_face {
        CubeFace::PosY | CubeFace::NegY | CubeFace::PosZ | CubeFace::NegZ => CubeAxis::X,
        CubeFace::PosX | CubeFace::NegX => CubeAxis::Z,
    };

    let index_0 = hit_face_cell_u;

    let axis_1 = match hit_face {
        CubeFace::PosX | CubeFace::NegX | CubeFace::PosZ | CubeFace::NegZ => CubeAxis::Y,
        CubeFace::PosY | CubeFace::NegY => CubeAxis::Z,
    };

    let index_1 = hit_face_cell_v;

    commands.entity(cube.0).insert(PendingDrag {
        viewport_origin,
        axis_0,
        index_0,
        viewport_dir_0,
        axis_1,
        index_1,
        viewport_dir_1,
    });
}

fn cubie_drag_pending_system(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    cube: Single<
        (Entity, &PendingDrag),
        (With<Cube>, Without<ActiveDrag>, Without<ActiveCubeRotation>),
    >,
    cubies: Query<(Entity, &Cubie, &Transform)>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        commands.entity(cube.0).remove::<PendingDrag>();
        return;
    }

    let cursor_position = match window.cursor_position() {
        Some(pos) => pos,
        None => {
            commands.entity(cube.0).remove::<PendingDrag>();
            return;
        }
    };

    const EPS: f32 = 1e-2;

    let drag_vector = cursor_position - cube.1.viewport_origin;

    if drag_vector.length() < EPS {
        return;
    }

    let drag_dir_0_proj_length = drag_vector
        .project_onto_normalized(cube.1.viewport_dir_0)
        .length();

    let drag_dir_1_proj_length = drag_vector
        .project_onto_normalized(cube.1.viewport_dir_1)
        .length();

    if drag_dir_0_proj_length < EPS && drag_dir_1_proj_length < EPS {
        return;
    }

    let (axis, index, viewport_dir) = if drag_dir_0_proj_length > drag_dir_1_proj_length {
        (
            cube.1.axis_0.clone(),
            cube.1.index_0,
            cube.1.viewport_dir_0.clone(),
        )
    } else {
        (
            cube.1.axis_1.clone(),
            cube.1.index_1,
            cube.1.viewport_dir_1.clone(),
        )
    };

    match axis {
        CubeAxis::X => {
            for cubie in cubies.iter() {
                if cubie.1.position.0 == index {
                    commands.entity(cubie.0).insert(BeingDragged {
                        prev_rotation: cubie.2.rotation,
                    });
                }
            }
        }
        CubeAxis::Y => {
            for cubie in cubies.iter() {
                if cubie.1.position.1 == index {
                    commands.entity(cubie.0).insert(BeingDragged {
                        prev_rotation: cubie.2.rotation,
                    });
                }
            }
        }
        CubeAxis::Z => {
            for cubie in cubies.iter() {
                if cubie.1.position.2 == index {
                    commands.entity(cubie.0).insert(BeingDragged {
                        prev_rotation: cubie.2.rotation,
                    });
                }
            }
        }
    }

    commands.entity(cube.0).insert(ActiveDrag {
        axis,
        viewport_origin: cube.1.viewport_origin,
        viewport_dir,
        current_angle: 0.0,
    });

    commands.entity(cube.0).remove::<PendingDrag>();
}

fn cubie_drag_system(
    mut commands: Commands,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    cube: Single<
        (Entity, &mut ActiveDrag),
        (
            With<Cube>,
            Without<PendingDrag>,
            Without<ActiveCubeRotation>,
        ),
    >,
    mut dragged_cubies: Query<&mut Transform, With<BeingDragged>>,
) {
    if !mouse_buttons.pressed(MouseButton::Left) {
        commands.entity(cube.0).remove::<ActiveDrag>();
        commands.entity(cube.0).insert(ActiveCubeRotation {
            axis: cube.1.axis,
            current_angle: cube.1.current_angle.rem_euclid(2.0 * PI),
            target_rotations: ((cube.1.current_angle / (PI / 2.0)).round() as i32).rem_euclid(4)
                as u32,
        });
        return;
    }

    let cursor_position = match window.cursor_position() {
        Some(pos) => pos,
        None => {
            return;
        }
    };

    let (_cube_entity, mut active_drag) = cube.into_inner();

    const DRAG_ANGLE_SENSITIVITY: f32 = 0.01;

    let intended_drag_angle = {
        let to_cursor = cursor_position - active_drag.viewport_origin;
        to_cursor.dot(active_drag.viewport_dir) * DRAG_ANGLE_SENSITIVITY
    };

    let current_angle = active_drag.current_angle;

    let drag_angle = intended_drag_angle - current_angle;

    let rotation_axis = match active_drag.axis {
        CubeAxis::X => Vec3::X,
        CubeAxis::Y => Vec3::Y,
        CubeAxis::Z => Vec3::Z,
    };

    let rotation_center = Vec3::ZERO;

    let rotation_quat = Quat::from_axis_angle(rotation_axis, drag_angle);

    for mut cubie_transform in dragged_cubies.iter_mut() {
        cubie_transform.rotate_around(rotation_center, rotation_quat);
    }

    active_drag.current_angle = intended_drag_angle;
}

fn cubie_rotation_system(
    mut commands: Commands,
    time: Res<Time>,
    cube: Single<
        (Entity, &mut ActiveCubeRotation),
        (With<Cube>, Without<PendingDrag>, Without<ActiveDrag>),
    >,
    mut dragged_cubies: Query<(Entity, &mut Cubie, &mut Transform, &BeingDragged)>,
) {
    let (cube_entity, mut active_rotation) = cube.into_inner();

    const ROTATION_SPEED: f32 = PI;

    let target_angle = active_rotation.target_rotations as f32 * (PI / 2.0);
    let angle_diff = if active_rotation.target_rotations == 0 {
        let normal_angle_diff = 0.0 - active_rotation.current_angle;
        let wrapped_angle_diff = 2.0 * PI - active_rotation.current_angle;
        if normal_angle_diff.abs() < wrapped_angle_diff.abs() {
            normal_angle_diff
        } else {
            wrapped_angle_diff
        }
    } else {
        target_angle - active_rotation.current_angle
    };

    let delta_angle =
        angle_diff.abs().min(ROTATION_SPEED * time.delta_secs()) * angle_diff.signum();

    let rotation_axis = match active_rotation.axis {
        CubeAxis::X => Vec3::X,
        CubeAxis::Y => Vec3::Y,
        CubeAxis::Z => Vec3::Z,
    };

    let rotation_center = Vec3::ZERO;

    let rotation_quat = Quat::from_axis_angle(rotation_axis, delta_angle);

    for (_cubie_entity, _cubie_data, mut cubie_transform, _being_dragged) in
        dragged_cubies.iter_mut()
    {
        cubie_transform.rotate_around(rotation_center, rotation_quat);
    }

    active_rotation.current_angle += delta_angle;

    const EPS: f32 = 1e-3;

    if (active_rotation.current_angle - target_angle).abs() < EPS
        || ((active_rotation.current_angle + EPS).rem_euclid(2.0 * PI) - target_angle).abs() < EPS
    {
        let cubie_rotation_quat = Quat::from_axis_angle(
            match active_rotation.axis {
                CubeAxis::X => Vec3::X,
                CubeAxis::Y => Vec3::Y,
                CubeAxis::Z => Vec3::Z,
            },
            (PI / 2.0) * active_rotation.target_rotations as f32,
        );

        for (cubie_entity, mut cubie_data, mut cubie_transform, being_dragged) in
            dragged_cubies.iter_mut()
        {
            let (curr_x, curr_y, curr_z) = cubie_data.position;

            cubie_data.position = match active_rotation.axis {
                CubeAxis::X => match active_rotation.target_rotations {
                    1 => (curr_x, 2 - curr_z, curr_y),
                    2 => (curr_x, 2 - curr_y, 2 - curr_z),
                    3 => (curr_x, curr_z, 2 - curr_y),
                    _ => (curr_x, curr_y, curr_z),
                },
                CubeAxis::Y => match active_rotation.target_rotations {
                    1 => (curr_z, curr_y, 2 - curr_x),
                    2 => (2 - curr_x, curr_y, 2 - curr_z),
                    3 => (2 - curr_z, curr_y, curr_x),
                    _ => (curr_x, curr_y, curr_z),
                },
                CubeAxis::Z => match active_rotation.target_rotations {
                    1 => (2 - curr_y, curr_x, curr_z),
                    2 => (2 - curr_x, 2 - curr_y, curr_z),
                    3 => (curr_y, 2 - curr_x, curr_z),
                    _ => (curr_x, curr_y, curr_z),
                },
            };

            let (new_x, new_y, new_z) = cubie_data.position;

            cubie_transform.translation =
                (Vec3::new(new_x as f32, new_y as f32, new_z as f32) - 1.0) / 3.0;
            cubie_transform.rotation = cubie_rotation_quat.mul_quat(being_dragged.prev_rotation);
            commands.entity(cubie_entity).remove::<BeingDragged>();
        }

        commands.entity(cube_entity).remove::<ActiveCubeRotation>();
    }
}

fn colored_cube_mesh(per_face_colors: [[f32; 4]; 6]) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    let positions = vec![
        // Front
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
        // Back
        [-0.5, -0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, -0.5, -0.5],
        // Left
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, 0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, -0.5],
        // Right
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
        [0.5, -0.5, 0.5],
        // Top
        [-0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        // Bottom
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
    ];

    let colors = per_face_colors
        .iter()
        .flat_map(|color| vec![*color; 4])
        .collect::<Vec<_>>();

    let normals = vec![[0.0, 0.0, 1.0]; 4]
        .into_iter()
        .chain(vec![[0.0, 0.0, -1.0]; 4])
        .chain(vec![[-1.0, 0.0, 0.0]; 4])
        .chain(vec![[1.0, 0.0, 0.0]; 4])
        .chain(vec![[0.0, 1.0, 0.0]; 4])
        .chain(vec![[0.0, -1.0, 0.0]; 4])
        .collect::<Vec<_>>();

    let uvs = vec![
        // Front
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        // Back
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        // Left
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        // Right
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        // Top
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        // Bottom
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
    ];

    let indices = (0..6)
        .flat_map(|i| {
            let base = i * 4;
            [base, base + 1, base + 2, base, base + 2, base + 3]
        })
        .collect::<Vec<_>>();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

fn game_setup(
    mut commands: Commands,
    mut clear_color: ResMut<ClearColor>,
    assets: ResMut<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0), //Transform::from_xyz(-3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    /*commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::PI / 4.),
            ..default()
        },
    ));*/

    let cubie_border_texture = assets.load_with_settings(
        "textures/cubie_face.png",
        |settings: &mut ImageLoaderSettings| {
            let sampler = settings.sampler.get_or_init_descriptor();
            sampler.address_mode_u = ImageAddressMode::ClampToEdge;
            sampler.address_mode_v = ImageAddressMode::ClampToEdge;
            sampler.min_filter = ImageFilterMode::Linear;
            sampler.mag_filter = ImageFilterMode::Linear;
        },
    );

    let cubie_material = materials.add(StandardMaterial {
        base_color_texture: Some(cubie_border_texture),
        unlit: true,
        ..Default::default()
    });

    commands
        .spawn((
            Cube,
            Visibility::Inherited,
            Transform::from_rotation(Quat::from_euler(
                EulerRot::XYZ,
                /*PI / 6.0*/ 30.0_f32.to_radians(),
                -PI / 4.0,
                0.0,
            )),
        ))
        .with_children(|parent| {
            for x in 0..3 {
                for y in 0..3 {
                    for z in 0..3 {
                        if x == 1 && y == 1 && z == 1 {
                            continue; // Skip the center cubie
                        }

                        parent.spawn((
                            Cubie {
                                position: (x, y, z),
                            },
                            Mesh3d(meshes.add(colored_cube_mesh([
                                if z == 2 {
                                    [1.0, 0.0, 0.0, 1.0] // Red
                                } else {
                                    [0.0, 0.0, 0.0, 1.0]
                                },
                                if z == 0 {
                                    [1.0, 0.2, 0.0, 1.0] // Orange
                                } else {
                                    [0.0, 0.0, 0.0, 1.0]
                                },
                                if x == 0 {
                                    [1.0, 1.0, 0.0, 1.0] // Yellow
                                } else {
                                    [0.0, 0.0, 0.0, 1.0]
                                },
                                if x == 2 {
                                    [1.0, 1.0, 1.0, 1.0] // White
                                } else {
                                    [0.0, 0.0, 0.0, 1.0]
                                },
                                if y == 2 {
                                    [0.0, 1.0, 0.0, 1.0] // Green
                                } else {
                                    [0.0, 0.0, 0.0, 1.0]
                                },
                                if y == 0 {
                                    [0.0, 0.0, 1.0, 1.0] // Blue
                                } else {
                                    [0.0, 0.0, 0.0, 1.0]
                                },
                            ]))),
                            MeshMaterial3d(cubie_material.clone()),
                            Transform {
                                translation: (Vec3::new(x as f32, y as f32, z as f32) - 1.0) / 3.0,
                                scale: Vec3::splat(1.0 / 3.0),
                                ..default()
                            },
                        ));
                    }
                }
            }
        });

    clear_color.0 = CLEAR_COLOR;
}

fn game_cleanup(mut _commands: Commands, mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = ClearColor::default().0;
}
