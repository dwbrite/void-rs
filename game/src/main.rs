// mod dialogue;
// mod graphics;
// mod resources;
// mod systems;

mod input;
mod player;
mod systems;

use bevy::{camera::{RenderTarget, visibility::RenderLayers}, color::palettes::css::GRAY, prelude::*, render::render_resource::Extent3d, window::WindowResized};
use bevy::asset::ErasedAssetLoader;
use bevy::render::render_resource::{
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// In-game resolution width.
const RES_WIDTH: u32 = 640 / 2;

/// In-game resolution height.
const RES_HEIGHT: u32 = 360 / 2;

/// Default render layers for pixel-perfect rendering.
/// You can skip adding this component, as this is the default.
const PIXEL_PERFECT_LAYERS: RenderLayers = RenderLayers::layer(0);

/// Render layers for high-resolution rendering.
const HIGH_RES_LAYERS: RenderLayers = RenderLayers::layer(1);

/// Low-resolution texture that contains the pixel-perfect world.
/// Canvas itself is rendered to the high-resolution world.
#[derive(Component)]
struct Canvas;

/// Camera that renders the pixel-perfect world to the [`Canvas`].
#[derive(Component)]
struct InGameCamera;

/// Camera that renders the [`Canvas`] (and other graphics on [`HIGH_RES_LAYERS`]) to the screen.
#[derive(Component)]
struct OuterCamera;

#[derive(Component)]
struct Rotate;

#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Component, Deref, DerefMut)]
struct Control(Controller);

enum Controller {
    Player,
    Scripted,
}

enum PlayerMovement {
    Idle,
    Running,
    Jumping,
    Falling,
}


use bevy::prelude::*;
use bevy::dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig};
use bevy_aseprite_ultra::prelude::*;
use bevy_rapier2d::na::DimAdd;
use bevy_rapier2d::prelude::*;
use player::PlayerPlugin;
use crate::input::InputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(FpsOverlayPlugin {
            config: FpsOverlayConfig {
                text_config: TextFont {
                    // Here we define size of our overlay
                    font_size: 12.0,
                    // If we want, we can use a custom font
                    font: default(),
                    ..default()
                },
                // We can also change color of the overlay
                text_color: Color::srgb(0.1, 0.7, 0.1),
                // We can also set the refresh interval for the FPS counter
                refresh_interval: core::time::Duration::from_millis(100),
                enabled: true,
                frame_time_graph_config: FrameTimeGraphConfig {
                    enabled: true,
                    // The minimum acceptable fps
                    min_fps: 30.0,
                    // The target fps
                    target_fps: 144.0,
                },
            },
        })
        .add_plugins(InputPlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(26.0))
        // .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(AsepriteUltraPlugin)
        .insert_resource(Time::<Fixed>::from_hz(128.0))
        .add_systems(Startup, (setup_camera, setup_movement_demo))
        .add_systems(Update, fit_canvas)
        .init_asset::<Aseprite>()
        .run();
}

fn setup_movement_demo(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>) {
    commands.spawn((
        PIXEL_PERFECT_LAYERS,
        // Keep floor visuals behind gameplay sprites on the same render layer.
        Transform::from_xyz(0.0, -60.0, -10.0),
        Mesh2d(meshes.add(Rectangle::new(640.0, 1.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.122, 0.082, 0.247))),
        Collider::cuboid(320.0, 0.5),
        RigidBody::Fixed,
        // AseAnimation {
        //     animation: "tag".into(),
        //     aseprite: asset_server.load("test-backdrop.aseprite"),
        // },
        // AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        // Sprite::default(),
    ));
}


fn setup_camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let canvas_size = Extent3d {
        width: RES_WIDTH,
        height: RES_HEIGHT,
        ..default()
    };

    // This Image serves as a canvas representing the low-resolution game screen
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: canvas_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // Fill image.data with zeroes
    canvas.resize(canvas_size);

    let image_handle = images.add(canvas);

    // This camera renders whatever is on `PIXEL_PERFECT_LAYERS` to the canvas
    commands.spawn((
        Camera2d,
        Camera {
            // Render before the "main pass" camera
            order: -1,
            clear_color: ClearColorConfig::Custom(Srgba::hex("0c0710").unwrap().into()),
            ..default()
        },
        RenderTarget::Image(image_handle.clone().into()),
        Msaa::Off,
        InGameCamera,
        PIXEL_PERFECT_LAYERS,
    ));

    // Spawn the canvas
    commands.spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYERS));

    // The "outer" camera renders whatever is on `HIGH_RES_LAYERS` to the screen.
    // here, the canvas and one of the sample sprites will be rendered by this camera
    commands.spawn((Camera2d, Msaa::Off, OuterCamera, HIGH_RES_LAYERS));
}


/// Scales camera projection to fit the window (integer multiples only).
fn fit_canvas(
    mut resize_messages: MessageReader<WindowResized>,
    mut projection: Single<&mut Projection, With<OuterCamera>>,
) {
    let Projection::Orthographic(projection) = &mut **projection else {
        return;
    };
    for window_resized in resize_messages.read() {
        let h_scale = window_resized.width / RES_WIDTH as f32;
        let v_scale = window_resized.height / RES_HEIGHT as f32;
        projection.scale = 1. / h_scale.min(v_scale).round();
    }
}



// fn main() {
//     let event_loop = EventLoop::new();

//     let title = "void";
//     let window = winit::window::WindowBuilder::new()
//         .with_title(title)
//         .with_inner_size(PhysicalSize::new(512, 288))
//         .with_resizable(false)
//         .build(&event_loop)
//         .unwrap();

//     use futures::executor::block_on;

//     let mut state = block_on(GameSystem::new(window));

//     event_loop.run(move |event, _, control_flow| {
//         *control_flow = ControlFlow::Poll;
//         match event {
//             Event::MainEventsCleared => {
//                 // simple fixed gameloop, we're going to let vsync handle the framerate
//                 state.update();
//                 state.draw();
//                 state.render();

//                 // TODO: sleep if frames are too fast
//             }
//             Event::WindowEvent {
//                 ref event,
//                 window_id,
//             } if window_id == state.gc.window.id() => {
//                 if !state.handle_input_events(event) {
//                     match event {
//                         WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
//                         _ => {}
//                     }
//                 }
//             }
//             _ => {}
//         }
//     });
// }
