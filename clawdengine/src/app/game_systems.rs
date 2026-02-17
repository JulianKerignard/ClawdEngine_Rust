use crate::animation;
use crate::physics;
use crate::scripting;
use crate::skeletal_animation;

use crate::App;

/// One-time flag to log play mode entry
static PLAY_MODE_LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

impl App {
    pub(crate) fn run_game_systems(&mut self, dt: f32) {
        let is_playing = self.editor_ctx.as_ref().is_some_and(|ec| ec.play_mode);

        if is_playing && !PLAY_MODE_LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            log::info!("[GameSystems] Play mode started — dt={:.4}s, entities={}",
                dt, self.world.entity_count());
        }
        if !is_playing {
            PLAY_MODE_LOGGED.store(false, std::sync::atomic::Ordering::Relaxed);
        }

        // Physics step (Play mode only)
        if is_playing {
            let (gravity, ground_y) = self.editor_ctx.as_ref()
                .map(|ec| (ec.project_settings.physics.gravity, ec.project_settings.physics.ground_y))
                .unwrap_or((physics::DEFAULT_GRAVITY, physics::DEFAULT_GROUND_Y));
            self.collision_events = physics::PhysicsSystem::step(
                &mut self.world,
                &mut self.collision_state,
                dt.clamp(0.001, 0.02),
                gravity,
                ground_y,
                &mut self.entity_buf,
            );
        }

        // Audio update (Play mode)
        if let Some(ref mut audio_sys) = self.audio {
            audio_sys.update(&mut self.world, is_playing);
        }

        // Animation system (Play mode only)
        if is_playing {
            animation::animation_system(&mut self.world, dt.max(0.001), &mut self.entity_buf);
        }

        // Skeletal animation (always evaluate for bind pose, only advance time when playing)
        if let Some(scene) = &mut self.scene {
            let anim_dt = if is_playing { dt.max(0.001) } else { 0.0 };
            log::trace!("[GameSystems] Skeletal animation tick: anim_dt={:.4}, skeletons_in_store={}, clips_in_store={}",
                anim_dt, scene.skeleton_store.len(), scene.animation_clip_store.len());

            // Borrow splitting: take buffers out temporarily
            let mut results_buf = std::mem::take(&mut scene.anim_results_buf);
            let mut local_buf = std::mem::take(&mut scene.anim_local_poses_buf);
            let mut global_buf = std::mem::take(&mut scene.anim_global_buf);
            let mut joint_buf = std::mem::take(&mut scene.anim_joint_buf);

            skeletal_animation::skeletal_animation_system(
                &mut self.world,
                anim_dt,
                &scene.skeleton_store,
                &scene.animation_clip_store,
                &scene.animator_controller_store,
                &mut self.entity_buf,
                &mut results_buf,
                &mut local_buf,
                &mut global_buf,
                &mut joint_buf,
            );

            scene.joint_matrix_cache.clear();
            for &(eid, data) in &results_buf {
                scene.joint_matrix_cache.insert(eid, data);
            }
            if !results_buf.is_empty() {
                log::trace!("[GameSystems] Updated {} skinned entity joint matrices", results_buf.len());
            }

            // Put buffers back
            scene.anim_results_buf = results_buf;
            scene.anim_local_poses_buf = local_buf;
            scene.anim_global_buf = global_buf;
            scene.anim_joint_buf = joint_buf;
        }

        // Script execution (Play mode only)
        if is_playing {
            self.run_scripts(dt);
        }
    }

    fn run_scripts(&mut self, dt: f32) {
        let dt_clamped = dt.max(0.001);
        let mut scripts = std::mem::take(&mut self.scripts);
        let mut pending_destroy = Vec::new();

        // Extract mesh_store reference for mesh-AABB raycasting in scripts.
        // Safety: mesh_store lives inside scene which isn't mutated by scripts.
        let mesh_store_ptr: Option<*const crate::renderer::mesh::MeshStore> =
            self.scene.as_ref().map(|s| &s.mesh_store as *const _);

        if !self.scripts_started {
            self.play_time = 0.0;
            for (eid, script) in &mut scripts {
                let mut ctx = scripting::ScriptContext::new_full(
                    *eid, &mut self.world, &[],
                    &self.input, 0.0, dt_clamped,
                );
                // SAFETY: mesh_store is not mutated while ScriptContext is alive
                if let Some(ptr) = mesh_store_ptr {
                    ctx = ctx.with_mesh_store(unsafe { &*ptr });
                }
                script.start(&mut ctx);
            }
            self.scripts_started = true;
        }

        self.play_time += dt_clamped;
        for (eid, script) in &mut scripts {
            let mut ctx = scripting::ScriptContext::new_full(
                *eid, &mut self.world, &self.collision_events,
                &self.input, self.play_time, dt_clamped,
            );
            // SAFETY: mesh_store is not mutated while ScriptContext is alive
            if let Some(ptr) = mesh_store_ptr {
                ctx = ctx.with_mesh_store(unsafe { &*ptr });
            }
            script.update(&mut ctx, dt_clamped);
            pending_destroy.extend(ctx.take_pending_destroy());
        }

        self.scripts = scripts;

        // Apply deferred entity destruction
        for eid in pending_destroy {
            if self.world.is_alive(eid) {
                self.world.destroy_entity(eid);
            }
            self.scripts.retain(|(id, _)| *id != eid);
        }
    }
}
