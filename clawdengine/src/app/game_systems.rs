use crate::animation;
use crate::physics;
use crate::scripting;
use crate::skeletal_animation;

use crate::App;

impl App {
    pub(crate) fn run_game_systems(&mut self, dt: f32) {
        let is_playing = self.editor_ctx.as_ref().is_some_and(|ec| ec.play_mode);

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
            );
        }

        // Audio update (Play mode)
        if let Some(ref mut audio_sys) = self.audio {
            audio_sys.update(&mut self.world, is_playing);
        }

        // Animation system (Play mode only)
        if is_playing {
            animation::animation_system(&mut self.world, dt.max(0.001));
        }

        // Skeletal animation (always evaluate for bind pose, only advance time when playing)
        if let Some(scene) = &mut self.scene {
            let anim_dt = if is_playing { dt.max(0.001) } else { 0.0 };
            let joint_data = skeletal_animation::skeletal_animation_system(
                &mut self.world,
                anim_dt,
                &scene.skeleton_store,
                &scene.animation_clip_store,
            );
            scene.joint_matrix_cache.clear();
            for (eid, data) in joint_data {
                scene.joint_matrix_cache.insert(eid, data);
            }
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

        if !self.scripts_started {
            self.play_time = 0.0;
            for (eid, script) in &mut scripts {
                let mut ctx = scripting::ScriptContext::new_full(
                    *eid, &mut self.world, &[],
                    &self.input, 0.0, dt_clamped,
                );
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
