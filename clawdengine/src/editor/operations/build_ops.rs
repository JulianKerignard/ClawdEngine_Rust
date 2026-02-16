use crate::editor::context::EditorContext;

pub(crate) fn process_build_game(ec: &mut EditorContext) {
    if !ec.pending_build_game {
        return;
    }
    ec.pending_build_game = false;

    let Some(ref project_path) = ec.current_project_path else {
        ec.save_feedback = Some(("Open a project first".into(), 3.0));
        return;
    };
    let project_path = project_path.clone();

    let manifest_name = crate::assets::project::load_manifest(&project_path)
        .map(|m| m.name)
        .unwrap_or_else(|_| "Game".to_string());
    let game_name = {
        let custom = &ec.project_settings.build.app_name;
        if custom.is_empty() { manifest_name } else { custom.clone() }
    };
    let scene_name = ec.scene_name.clone();

    if let Ok(mut manifest) = crate::assets::project::load_manifest(&project_path) {
        manifest.default_scene = Some(scene_name.clone());
        let _ = crate::assets::project::save_manifest(&project_path, &manifest);
    }

    let safe_name = game_name.replace(' ', "");
    let export_dir = std::path::Path::new(&project_path).join("export");
    let app_dir = export_dir.join(format!("{}.app", safe_name));
    let contents_dir = app_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let resources_dir = contents_dir.join("Resources");

    let _ = std::fs::remove_dir_all(&app_dir);
    if let Err(e) = std::fs::create_dir_all(&macos_dir) {
        ec.save_feedback = Some((format!("Build failed: {}", e), 3.0));
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&resources_dir) {
        ec.save_feedback = Some((format!("Build failed: {}", e), 3.0));
        return;
    }

    let exe_path = match std::env::current_exe().and_then(|p| p.canonicalize()) {
        Ok(p) => p,
        Err(e) => {
            ec.save_feedback = Some((format!("Cannot locate binary: {}", e), 3.0));
            return;
        }
    };
    let bin_dst = macos_dir.join("clawdengine");
    if let Err(e) = std::fs::copy(&exe_path, &bin_dst) {
        ec.save_feedback = Some((format!("Copy binary failed: {}", e), 3.0));
        return;
    }

    let proj = std::path::Path::new(&project_path);
    for dir_name in &["scenes", "meshes", "textures", "audio"] {
        let src = proj.join(dir_name);
        if src.is_dir() {
            copy_tree(&src, &resources_dir.join(dir_name));
        }
    }

    let game_ron = format!(
        "(\n    name: \"{}\",\n    startup_scene: \"scenes/{}.ron\",\n)\n",
        game_name, scene_name
    );
    let _ = std::fs::write(resources_dir.join("game.ron"), &game_ron);

    let bs = &ec.project_settings.build;
    let bundle_id = format!("{}.{}", bs.bundle_id_prefix, safe_name.to_lowercase());
    let version = &bs.version;
    let min_macos = &bs.min_macos_version;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleDisplayName</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bid}</string>
    <key>CFBundleVersion</key>
    <string>{ver}</string>
    <key>CFBundleShortVersionString</key>
    <string>{ver}</string>
    <key>CFBundleExecutable</key>
    <string>clawdengine</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>{min_os}</string>
</dict>
</plist>"#,
        name = game_name,
        bid = bundle_id,
        ver = version,
        min_os = min_macos,
    );
    let _ = std::fs::write(contents_dir.join("Info.plist"), &plist);

    let _ = std::process::Command::new("codesign")
        .args(["--force", "-s", "-"])
        .arg(&bin_dst)
        .output();
    let _ = std::process::Command::new("codesign")
        .args(["--force", "-s", "-"])
        .arg(&app_dir)
        .output();

    let app_size = fs_dir_size(&app_dir);
    let size_str = if app_size > 1_000_000 {
        format!("{:.1} MB", app_size as f64 / 1_000_000.0)
    } else {
        format!("{} KB", app_size / 1_000)
    };

    ec.save_feedback = Some((format!("Game built: {}.app ({})", safe_name, size_str), 5.0));
    log::info!("Game exported to {} ({})", app_dir.display(), size_str);
}

fn copy_tree(src: &std::path::Path, dst: &std::path::Path) {
    let _ = std::fs::create_dir_all(dst);
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let s = entry.path();
            let d = dst.join(entry.file_name());
            if s.is_dir() {
                copy_tree(&s, &d);
            } else {
                let _ = std::fs::copy(&s, &d);
            }
        }
    }
}

fn fs_dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += fs_dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}
