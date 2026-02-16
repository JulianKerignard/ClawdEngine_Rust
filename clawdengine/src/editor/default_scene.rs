use crate::core::{self, EntityId, World};
use crate::scripting::{self, GameScript};

/// Creates the showcase scene with 9 entities and returns attached scripts.
pub fn setup_default_scene(
    world: &mut World,
    cube_mesh_id: usize,
    sphere_mesh_id: usize,
) -> Vec<(EntityId, Box<dyn GameScript>)> {
    // ---- Lights ----

    let sun = world.spawn_entity();
    world.set_name(sun, "Sun");
    world.set_transform(sun, core::Transform {
        position: glam::Vec3::new(0.0, 5.0, 0.0),
        rotation: glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            -50_f32.to_radians(),
            30_f32.to_radians(),
            0.0,
        ),
        ..Default::default()
    });
    world.set_light(sun, core::Light {
        kind: core::LightKind::Directional,
        color: glam::Vec3::new(1.0, 0.95, 0.85),
        intensity: 1.5,
        ..Default::default()
    });

    let point_light = world.spawn_entity();
    world.set_name(point_light, "PointLight");
    world.set_transform(point_light, core::Transform {
        position: glam::Vec3::new(-3.0, 3.0, 2.0),
        ..Default::default()
    });
    world.set_light(point_light, core::Light {
        kind: core::LightKind::Point,
        color: glam::Vec3::new(0.4, 0.6, 1.0),
        intensity: 1.5,
        ..Default::default()
    });

    let warm_light = world.spawn_entity();
    world.set_name(warm_light, "WarmLight");
    world.set_transform(warm_light, core::Transform {
        position: glam::Vec3::new(3.0, 2.0, -2.0),
        ..Default::default()
    });
    world.set_light(warm_light, core::Light {
        kind: core::LightKind::Point,
        color: glam::Vec3::new(1.0, 0.6, 0.2),
        intensity: 1.2,
        ..Default::default()
    });

    // ---- Ground ----

    let ground = world.spawn_entity();
    world.set_name(ground, "Ground");
    world.set_transform(ground, core::Transform {
        position: glam::Vec3::new(0.0, -0.05, 0.0),
        scale: glam::Vec3::new(12.0, 0.1, 12.0),
        ..Default::default()
    });
    world.set_material(ground, core::Material {
        albedo: glam::Vec3::new(0.4, 0.45, 0.4),
        roughness: 0.95,
        metallic: 0.0,
        ..Default::default()
    });
    world.set_mesh_renderer(ground, core::MeshRenderer {
        mesh_id: Some(cube_mesh_id),
        visible: true,
    });
    world.set_collider(ground, core::Collider::default());

    // ---- Scripted objects ----

    let cube = world.spawn_entity();
    world.set_name(cube, "Cube");
    world.set_transform(cube, core::Transform {
        position: glam::Vec3::new(0.0, 0.5, 0.0),
        ..Default::default()
    });
    world.set_material(cube, core::Material::default());
    world.set_mesh_renderer(cube, core::MeshRenderer {
        mesh_id: Some(cube_mesh_id),
        visible: true,
    });
    world.set_rigid_body(cube, core::RigidBody::default());
    world.set_collider(cube, core::Collider::default());

    let sphere = world.spawn_entity();
    world.set_name(sphere, "Sphere");
    world.set_transform(sphere, core::Transform {
        position: glam::Vec3::new(3.0, 1.0, 0.0),
        ..Default::default()
    });
    world.set_material(sphere, core::Material {
        albedo: glam::Vec3::new(0.3, 0.8, 0.4),
        ..Default::default()
    });
    world.set_mesh_renderer(sphere, core::MeshRenderer {
        mesh_id: Some(sphere_mesh_id),
        visible: true,
    });

    // ---- Static showcase objects ----

    let metal_sphere = world.spawn_entity();
    world.set_name(metal_sphere, "MetalSphere");
    world.set_transform(metal_sphere, core::Transform {
        position: glam::Vec3::new(-2.5, 0.5, 1.0),
        ..Default::default()
    });
    world.set_material(metal_sphere, core::Material {
        albedo: glam::Vec3::new(0.9, 0.9, 0.95),
        roughness: 0.1,
        metallic: 1.0,
        ..Default::default()
    });
    world.set_mesh_renderer(metal_sphere, core::MeshRenderer {
        mesh_id: Some(sphere_mesh_id),
        visible: true,
    });
    world.set_rigid_body(metal_sphere, core::RigidBody::default());
    world.set_collider(metal_sphere, core::Collider::default());

    let red_cube = world.spawn_entity();
    world.set_name(red_cube, "RedCube");
    world.set_transform(red_cube, core::Transform {
        position: glam::Vec3::new(-1.0, 0.5, -2.5),
        ..Default::default()
    });
    world.set_material(red_cube, core::Material {
        albedo: glam::Vec3::new(0.9, 0.15, 0.1),
        roughness: 0.3,
        metallic: 0.0,
        ..Default::default()
    });
    world.set_mesh_renderer(red_cube, core::MeshRenderer {
        mesh_id: Some(cube_mesh_id),
        visible: true,
    });
    world.set_rigid_body(red_cube, core::RigidBody::default());
    world.set_collider(red_cube, core::Collider::default());

    let pedestal = world.spawn_entity();
    world.set_name(pedestal, "PedestalCube");
    world.set_transform(pedestal, core::Transform {
        position: glam::Vec3::new(2.0, 0.15, -2.0),
        scale: glam::Vec3::new(0.8, 0.3, 0.8),
        ..Default::default()
    });
    world.set_material(pedestal, core::Material {
        albedo: glam::Vec3::new(0.7, 0.65, 0.5),
        roughness: 0.7,
        metallic: 0.0,
        ..Default::default()
    });
    world.set_mesh_renderer(pedestal, core::MeshRenderer {
        mesh_id: Some(cube_mesh_id),
        visible: true,
    });

    // ---- Audio Source ----

    let audio_entity = world.spawn_entity();
    world.set_name(audio_entity, "AudioChime");
    world.set_transform(audio_entity, core::Transform {
        position: glam::Vec3::new(0.0, 1.0, 0.0),
        ..Default::default()
    });
    world.set_audio_source(audio_entity, core::AudioSource {
        audio_path: Some(crate::assets::paths::resolve("assets/audio/test_chime.wav")
            .to_string_lossy().to_string()),
        volume: 0.8,
        pitch: 1.0,
        loop_audio: false,
        play_on_start: true,
        spatial: false,
        max_distance: 20.0,
        is_playing: false,
    });

    // ---- Canvas + UI Elements ----

    let canvas = world.spawn_entity();
    world.set_name(canvas, "Canvas");
    world.set_canvas(canvas, core::Canvas::default());

    let ui_title = world.spawn_entity();
    world.set_name(ui_title, "Title");
    world.set_ui_element(ui_title, core::UiElement {
        kind: core::UiElementKind::Text,
        text: "ClawdEngine".to_string(),
        font_size: 24.0,
        color: glam::Vec3::ONE,
        alpha: 0.9,
        anchor: core::UiAnchor::TopLeft,
        offset: [20.0, 20.0],
        ..Default::default()
    });
    world.set_parent(ui_title, canvas);

    let ui_panel = world.spawn_entity();
    world.set_name(ui_panel, "Info Panel");
    world.set_ui_element(ui_panel, core::UiElement {
        kind: core::UiElementKind::Panel,
        text: String::new(),
        color: glam::Vec3::new(0.1, 0.1, 0.2),
        alpha: 0.6,
        anchor: core::UiAnchor::BottomRight,
        offset: [-220.0, -80.0],
        size: [200.0, 60.0],
        ..Default::default()
    });
    world.set_parent(ui_panel, canvas);

    let ui_score = world.spawn_entity();
    world.set_name(ui_score, "Score");
    world.set_ui_element(ui_score, core::UiElement {
        kind: core::UiElementKind::Text,
        text: "Score: 0".to_string(),
        font_size: 16.0,
        color: glam::Vec3::new(1.0, 0.9, 0.3),
        alpha: 0.9,
        anchor: core::UiAnchor::BottomRight,
        offset: [-210.0, -60.0],
        ..Default::default()
    });
    world.set_parent(ui_score, canvas);

    // ---- Main Camera ----

    let main_camera = world.spawn_entity();
    world.set_name(main_camera, "MainCamera");
    world.set_transform(main_camera, core::Transform {
        position: glam::Vec3::new(0.0, 2.0, 5.0),
        rotation: glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            -15_f32.to_radians(),
            0.0,
            0.0,
        ),
        ..Default::default()
    });
    world.set_camera(main_camera, core::CameraComponent::default());
    world.set_audio_listener(main_camera, core::AudioListener::default());

    // ---- Scripts ----

    vec![
        (cube, Box::new(scripting::RotateAndPulse::new()) as Box<dyn GameScript>),
        (sphere, Box::new(scripting::Oscillate::new())),
        (metal_sphere, Box::new(scripting::GravityBounce::new())),
        (pedestal, Box::new(scripting::ColorCycle::new())),
        (audio_entity, Box::new(scripting::AudioDemo::new())),
        (main_camera, Box::new(scripting::PlayerHUD::new())),
    ]
}
