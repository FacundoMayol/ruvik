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

#[derive(Component)]
struct Cubie(u32, u32, u32);

const CLEAR_COLOR: Color = Color::srgb(0.40, 0.36, 0.23);

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Game), game_setup)
        .add_systems(
            Update,
            (camera_rotation_system).run_if(in_state(GameState::Game)),
        )
        .add_systems(OnExit(GameState::Game), game_cleanup);
}

fn camera_rotation_system(
    mut motion_events: MessageReader<MouseMotion>,
    mut camera_transform: Single<&mut Transform, With<Camera3d>>,
) {
    let mut delta = Vec2::ZERO;
    for event in motion_events.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    let yaw = Quat::from_rotation_y(-delta.x * 0.005);
    let pitch = Quat::from_rotation_x(-delta.y * 0.005);

    let direction = camera_transform.translation.normalize();
    let distance = camera_transform.translation.length();

    let new_direction = yaw * direction;
    let new_direction = pitch * new_direction;

    camera_transform.translation = new_direction * distance;
    camera_transform.look_at(Vec3::ZERO, Vec3::Y);
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
        Transform::from_xyz(-3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
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

    //commands.spawn((Cube,)).with_children(|parent| {
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                commands.spawn((
                    //parent.spawn((
                    Cubie(x, y, z),
                    Mesh3d(meshes.add(colored_cube_mesh([
                        if z == 0 {
                            [1.0, 0.0, 0.0, 1.0] // Red
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        },
                        if z == 2 {
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
                        if y == 0 {
                            [0.0, 1.0, 0.0, 1.0] // Green
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        },
                        if y == 2 {
                            [0.0, 0.0, 1.0, 1.0] // Blue
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        },
                    ]))),
                    MeshMaterial3d(cubie_material.clone()),
                    Transform {
                        translation: Vec3::new(
                            (x as f32 - 1.0) / 3.0,
                            (1.0 - y as f32) / 3.0,
                            (1.0 - z as f32) / 3.0,
                        ),
                        scale: Vec3::splat(1.0 / 3.0),
                        ..default()
                    },
                ));
            }
        }
    }
    //});

    clear_color.0 = CLEAR_COLOR;
}

fn game_cleanup(mut _commands: Commands, mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = ClearColor::default().0;
}
