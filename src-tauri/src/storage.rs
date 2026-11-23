use crate::timer::{validate_snapshot, Engine, Snapshot, MAX_STATE_BYTES};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const STATE_FILE: &str = "focus-state.json";

#[cfg(any(not(target_os = "android"), test))]
pub struct LoadResult {
    pub engine: Engine,
    pub recovered_corrupt: Option<PathBuf>,
}

#[cfg(any(target_os = "android", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AndroidClock {
    /// -1 means the OS could not supply a reliable boot identity.
    pub boot_count: i64,
    pub elapsed_realtime_ms: i64,
}

#[cfg(any(target_os = "android", test))]
impl AndroidClock {
    pub fn validate(self) -> Result<(), String> {
        if !(-1..=i64::from(u32::MAX)).contains(&self.boot_count)
            || !(0..=9_007_199_254_740_991).contains(&self.elapsed_realtime_ms)
        {
            return Err("invalid_android_clock".into());
        }
        Ok(())
    }
}

#[cfg(any(target_os = "android", test))]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AndroidEnvelope {
    android_version: u8,
    clock: AndroidClock,
    snapshot: Snapshot,
}

#[cfg(any(target_os = "android", test))]
pub struct AndroidLoadResult {
    pub engine: Engine,
    pub recovered_corrupt: Option<PathBuf>,
    /// Epoch-shaped time calibrated to elapsedRealtime, not a second timer.
    pub logical_now: i64,
}

/// One Store must be protected by the runtime's persistence mutex.
/// Engine locks are released before calling either filesystem method.
pub struct Store {
    directory: PathBuf,
    path: PathBuf,
    loaded: bool,
    write_blocked: bool,
    last_saved_revision: Option<u64>,
}

impl Store {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: app_data_dir.join(STATE_FILE),
            directory: app_data_dir,
            loaded: false,
            write_blocked: true,
            last_saved_revision: None,
        }
    }

    fn ensure_directory(&self) -> Result<(), String> {
        fs::create_dir_all(&self.directory)
            .map_err(|e| format!("storage_directory_failed: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.directory, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("storage_permissions_failed: {e}"))?;
        }
        Ok(())
    }

    fn read_state(&mut self) -> Result<Option<Vec<u8>>, String> {
        self.loaded = true;
        self.write_blocked = true;
        self.last_saved_revision = None;
        self.ensure_directory()?;
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                self.write_blocked = false;
                return Ok(None);
            }
            Err(error) => return Err(format!("storage_read_failed: {error}")),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("storage_path_not_regular".into());
        }
        let mut bytes = Vec::new();
        File::open(&self.path)
            .map_err(|e| format!("storage_read_failed: {e}"))?
            .take((MAX_STATE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("storage_read_failed: {e}"))?;
        Ok(Some(bytes))
    }

    fn accept(&mut self, revision: u64) {
        self.last_saved_revision = Some(revision);
        self.write_blocked = false;
    }

    fn preserve_corrupt(&mut self, now: i64) -> Result<PathBuf, String> {
        let quarantine = self.directory.join(format!(
            "focus-state.corrupt-{}-{}.json",
            now.max(0),
            Uuid::new_v4()
        ));
        fs::rename(&self.path, &quarantine)
            .map_err(|e| format!("corrupt_state_preservation_failed: {e}"))?;
        sync_directory(&self.directory)
            .map_err(|e| format!("storage_directory_sync_failed: {e}"))?;
        self.write_blocked = false;
        Ok(quarantine)
    }

    #[cfg(any(not(target_os = "android"), test))]
    pub fn load(&mut self, now: i64) -> Result<LoadResult, String> {
        let Some(bytes) = self.read_state()? else {
            return Ok(LoadResult {
                engine: Engine::new(now),
                recovered_corrupt: None,
            });
        };
        if let Some(raw) = bounded_utf8(&bytes) {
            reject_future_schema(raw)?;
            // Android envelopes are deliberately not silently opened by the
            // desktop recovery policy or replaced during an accidental downgrade.
            if serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .is_some_and(|value| value.get("androidVersion").is_some())
            {
                return Err("platform_state_mismatch".into());
            }
            if let Ok(engine) = Engine::restore(raw, now) {
                let stored: Snapshot =
                    serde_json::from_str(raw).map_err(|_| String::from("invalid_state"))?;
                self.accept(stored.revision);
                return Ok(LoadResult {
                    engine,
                    recovered_corrupt: None,
                });
            }
        }
        let quarantine = self.preserve_corrupt(now)?;
        Ok(LoadResult {
            engine: Engine::new(now),
            recovered_corrupt: Some(quarantine),
        })
    }

    #[cfg(any(target_os = "android", test))]
    pub fn load_android(
        &mut self,
        wall_now: i64,
        clock: AndroidClock,
        force_pause: bool,
    ) -> Result<AndroidLoadResult, String> {
        clock.validate()?;
        let Some(bytes) = self.read_state()? else {
            return Ok(AndroidLoadResult {
                engine: Engine::new(wall_now),
                recovered_corrupt: None,
                logical_now: wall_now,
            });
        };
        if let Some(raw) = bounded_utf8(&bytes) {
            reject_future_schema(raw)?;
            let parsed = serde_json::from_str::<serde_json::Value>(raw).ok();
            if parsed
                .as_ref()
                .is_some_and(|value| value.get("androidVersion").is_some())
            {
                if let Ok(saved) = serde_json::from_str::<AndroidEnvelope>(raw) {
                    if saved.android_version == 1
                        && saved.clock.validate().is_ok()
                        && validate_snapshot(&saved.snapshot).is_ok()
                    {
                        let encoded = serde_json::to_string(&saved.snapshot)
                            .map_err(|_| String::from("serialization_failed"))?;
                        let continuing = !force_pause
                            && clock.boot_count >= 0
                            && saved.clock.boot_count == clock.boot_count
                            && clock.elapsed_realtime_ms >= saved.clock.elapsed_realtime_ms
                            && saved.snapshot.timer.status == crate::timer::TimerStatus::Running;
                        let logical_now = if continuing {
                            saved
                                .snapshot
                                .timer
                                .updated_at
                                .checked_add(
                                    clock.elapsed_realtime_ms - saved.clock.elapsed_realtime_ms,
                                )
                                .ok_or("invalid_android_clock")?
                        } else {
                            wall_now
                        };
                        let engine = if continuing {
                            Engine::restore_continuing(&encoded, logical_now)
                        } else {
                            Engine::restore(&encoded, logical_now)
                        };
                        if let Ok(engine) = engine {
                            self.accept(saved.snapshot.revision);
                            return Ok(AndroidLoadResult {
                                engine,
                                recovered_corrupt: None,
                                logical_now,
                            });
                        }
                    }
                }
            } else if let Ok(engine) = Engine::restore(raw, wall_now) {
                // Older v1 files have no boot authorization, so pause safely.
                let stored: Snapshot =
                    serde_json::from_str(raw).map_err(|_| String::from("invalid_state"))?;
                self.accept(stored.revision);
                return Ok(AndroidLoadResult {
                    engine,
                    recovered_corrupt: None,
                    logical_now: wall_now,
                });
            }
        }
        let quarantine = self.preserve_corrupt(wall_now)?;
        Ok(AndroidLoadResult {
            engine: Engine::new(wall_now),
            recovered_corrupt: Some(quarantine),
            logical_now: wall_now,
        })
    }

    #[cfg(any(not(target_os = "android"), test))]
    pub fn save(&mut self, revision: u64, raw: &str) -> Result<(), String> {
        if !self.loaded || self.write_blocked {
            return Err("storage_write_blocked".into());
        }
        if raw.len() > MAX_STATE_BYTES {
            return Err("state_too_large".into());
        }
        let snapshot: Snapshot =
            serde_json::from_str(raw).map_err(|_| String::from("invalid_state"))?;
        validate_snapshot(&snapshot)?;
        if snapshot.revision != revision {
            return Err("state_revision_mismatch".into());
        }
        self.write_atomic(revision, raw)
    }

    #[cfg(any(target_os = "android", test))]
    pub fn save_android(&mut self, snapshot: &Snapshot, clock: AndroidClock) -> Result<(), String> {
        clock.validate()?;
        validate_snapshot(snapshot)?;
        let raw = serde_json::to_string(&AndroidEnvelope {
            android_version: 1,
            clock,
            snapshot: snapshot.clone(),
        })
        .map_err(|_| String::from("serialization_failed"))?;
        self.write_atomic(snapshot.revision, &raw)
    }

    fn write_atomic(&mut self, revision: u64, raw: &str) -> Result<(), String> {
        if !self.loaded || self.write_blocked {
            return Err("storage_write_blocked".into());
        }
        if raw.len() > MAX_STATE_BYTES {
            return Err("state_too_large".into());
        }
        if self
            .last_saved_revision
            .is_some_and(|saved| revision <= saved)
        {
            return Ok(());
        }
        self.ensure_directory()?;
        let temp = self
            .directory
            .join(format!(".focus-state-{}.tmp", Uuid::new_v4()));
        let result = (|| -> Result<(), String> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temp)
                .map_err(|e| format!("storage_create_failed: {e}"))?;
            file.write_all(raw.as_bytes())
                .map_err(|e| format!("storage_write_failed: {e}"))?;
            file.sync_all()
                .map_err(|e| format!("storage_file_sync_failed: {e}"))?;
            drop(file);
            fs::rename(&temp, &self.path).map_err(|e| format!("storage_replace_failed: {e}"))?;
            self.last_saved_revision = Some(revision);
            sync_directory(&self.directory)
                .map_err(|e| format!("storage_directory_sync_failed: {e}"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

fn bounded_utf8(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes)
        .ok()
        .filter(|_| bytes.len() <= MAX_STATE_BYTES)
}

fn reject_future_schema(raw: &str) -> Result<(), String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        let future = ["version", "androidVersion"].iter().any(|key| {
            value
                .get(key)
                .and_then(|v| v.as_u64())
                .is_some_and(|v| v > 1)
        }) || value
            .get("snapshot")
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v > 1);
        if future {
            return Err("unsupported_state_version".into());
        }
    }
    Ok(())
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::{TimerAction, TimerStatus, CHECKPOINT_MS};
    const NOW: i64 = 1_800_000_000_000;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("focus-store-test-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn store(&self) -> Store {
            Store::new(self.0.clone())
        }
        fn state_path(&self) -> PathBuf {
            self.0.join(STATE_FILE)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn android_running_fixture(dir: &Fixture) -> (Snapshot, AndroidClock) {
        let clock = AndroidClock {
            boot_count: 7,
            elapsed_realtime_ms: 100_000,
        };
        let mut store = dir.store();
        let mut engine = store.load_android(NOW, clock, false).unwrap().engine;
        engine
            .dispatch(TimerAction::SetDuration { minutes: 1 }, NOW)
            .unwrap();
        engine.dispatch(TimerAction::Start, NOW).unwrap();
        engine.tick(NOW + 10_000);
        let snapshot = engine.snapshot();
        store.save_android(&snapshot, clock).unwrap();
        (snapshot, clock)
    }

    #[test]
    fn android_same_boot_recovers_monotonic_deadline_despite_wall_clock_changes() {
        let dir = Fixture::new();
        let (saved, clock) = android_running_fixture(&dir);
        let observed = AndroidClock {
            elapsed_realtime_ms: clock.elapsed_realtime_ms + 15_000,
            ..clock
        };
        let mut store = dir.store();
        let loaded = store
            .load_android(NOW + 7_200_000, observed, false)
            .unwrap();
        assert!(loaded.recovered_corrupt.is_none());
        assert_eq!(loaded.logical_now, NOW + 25_000);
        let mut engine = loaded.engine;
        let result = engine.tick(loaded.logical_now);
        assert_eq!(result.snapshot.timer.status, TimerStatus::Running);
        assert_eq!(result.snapshot.timer.remaining_ms, 35_000);
        assert_eq!(result.snapshot.timer.run_id, saved.timer.run_id);
        assert_eq!(result.snapshot.timer.deadline_at, Some(NOW + 60_000));
        assert!(result.snapshot.history.is_empty());
    }

    #[test]
    fn android_late_alarm_completes_once_and_atomic_reopen_does_not_repeat() {
        let dir = Fixture::new();
        let (saved, clock) = android_running_fixture(&dir);
        let observed = AndroidClock {
            elapsed_realtime_ms: clock.elapsed_realtime_ms + 120_000,
            ..clock
        };
        let mut store = dir.store();
        let loaded = store.load_android(NOW + 130_000, observed, false).unwrap();
        let mut engine = loaded.engine;
        let result = engine.tick(loaded.logical_now);
        assert_eq!(
            result.completed.as_ref().unwrap().id,
            saved.timer.run_id.unwrap()
        );
        assert_eq!(result.completed.unwrap().finished_at, NOW + 60_000);
        store.save_android(&engine.snapshot(), observed).unwrap();
        let mut reopened = dir
            .store()
            .load_android(
                NOW + 131_000,
                AndroidClock {
                    elapsed_realtime_ms: observed.elapsed_realtime_ms + 1_000,
                    ..clock
                },
                false,
            )
            .unwrap()
            .engine;
        let after = reopened.tick(NOW + 131_000);
        assert!(after.completed.is_none());
        assert_eq!(after.snapshot.history.len(), 1);
        assert_eq!(after.snapshot.daily_stats[0].completed_sessions, 1);
    }

    #[test]
    fn android_reboot_unknown_boot_invalid_elapsed_or_explicit_stop_recovers_paused() {
        for (clock, force_pause) in [
            (
                AndroidClock {
                    boot_count: 8,
                    elapsed_realtime_ms: 5_000,
                },
                false,
            ),
            (
                AndroidClock {
                    boot_count: -1,
                    elapsed_realtime_ms: 120_000,
                },
                false,
            ),
            (
                AndroidClock {
                    boot_count: 7,
                    elapsed_realtime_ms: 50_000,
                },
                false,
            ),
            (
                AndroidClock {
                    boot_count: 7,
                    elapsed_realtime_ms: 120_000,
                },
                true,
            ),
        ] {
            let dir = Fixture::new();
            let (saved, _) = android_running_fixture(&dir);
            let loaded = dir
                .store()
                .load_android(NOW + 86_400_000, clock, force_pause)
                .unwrap();
            assert!(loaded.recovered_corrupt.is_none());
            assert_eq!(loaded.engine.snapshot().timer.status, TimerStatus::Paused);
            assert_eq!(
                loaded.engine.snapshot().timer.remaining_ms,
                saved.timer.remaining_ms
            );
            assert!(loaded.engine.snapshot().restored_paused);
            assert!(loaded.engine.snapshot().history.is_empty());
        }
    }

    #[test]
    fn android_legacy_v1_migrates_as_paused_and_future_envelope_is_preserved() {
        let dir = Fixture::new();
        let mut engine = Engine::new(NOW);
        engine.dispatch(TimerAction::Start, NOW).unwrap();
        fs::write(dir.state_path(), engine.serialize().unwrap()).unwrap();
        let clock = AndroidClock {
            boot_count: 7,
            elapsed_realtime_ms: 100_000,
        };
        let mut store = dir.store();
        let loaded = store.load_android(NOW + 10_000, clock, false).unwrap();
        assert_eq!(loaded.engine.snapshot().timer.status, TimerStatus::Paused);
        store
            .save_android(&loaded.engine.snapshot(), clock)
            .unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.state_path()).unwrap()).unwrap();
        assert_eq!(value["androidVersion"], 1);
        assert_eq!(value["clock"]["bootCount"], 7);
        value["androidVersion"] = 2.into();
        let future = serde_json::to_vec(&value).unwrap();
        fs::write(dir.state_path(), &future).unwrap();
        let mut blocked = dir.store();
        assert!(matches!(blocked.load_android(NOW, clock, false),
            Err(error) if error == "unsupported_state_version"));
        assert_eq!(
            blocked.save_android(&engine.snapshot(), clock).unwrap_err(),
            "storage_write_blocked"
        );
        assert_eq!(fs::read(dir.state_path()).unwrap(), future);
    }

    #[test]
    fn android_corrupt_clock_is_retained_and_stale_envelope_cannot_win() {
        let dir = Fixture::new();
        let (saved, clock) = android_running_fixture(&dir);
        let mut store = dir.store();
        let loaded = store.load_android(NOW + 10_000, clock, false).unwrap();
        let mut engine = loaded.engine;
        engine.dispatch(TimerAction::Pause, NOW + 15_000).unwrap();
        store.save_android(&engine.snapshot(), clock).unwrap();
        let latest = fs::read(dir.state_path()).unwrap();
        store.save_android(&saved, clock).unwrap();
        assert_eq!(fs::read(dir.state_path()).unwrap(), latest);
        let mut value: serde_json::Value = serde_json::from_slice(&latest).unwrap();
        value["clock"]["elapsedRealtimeMs"] = (-1).into();
        let corrupt = serde_json::to_vec(&value).unwrap();
        fs::write(dir.state_path(), &corrupt).unwrap();
        let recovered = dir.store().load_android(NOW, clock, false).unwrap();
        assert_eq!(
            fs::read(recovered.recovered_corrupt.unwrap()).unwrap(),
            corrupt
        );
        assert_eq!(recovered.engine.snapshot().timer.status, TimerStatus::Idle);
    }

    #[test]
    fn first_run_round_trip_and_running_restore_preserve_checkpoint() {
        let dir = Fixture::new();
        let mut store = dir.store();
        let loaded = store.load(NOW).unwrap();
        assert!(loaded.recovered_corrupt.is_none());
        let mut engine = loaded.engine;
        engine.dispatch(TimerAction::Start, NOW).unwrap();
        engine.tick(NOW + CHECKPOINT_MS);
        store
            .save(engine.snapshot().revision, &engine.serialize().unwrap())
            .unwrap();
        let mut reopened = dir.store();
        let restored = reopened.load(NOW + 86_400_000).unwrap().engine.snapshot();
        assert_eq!(restored.timer.status, TimerStatus::Paused);
        assert_eq!(restored.timer.remaining_ms, 25 * 60_000 - CHECKPOINT_MS);
        assert!(restored.restored_paused);
        assert!(restored.history.is_empty());
    }

    #[test]
    fn legacy_v1_file_loads_without_quarantine_and_saves_mini_timer_preferences() {
        let dir = Fixture::new();
        let mut original = Engine::new(NOW);
        original.dispatch(TimerAction::Start, NOW).unwrap();
        original.dispatch(TimerAction::Pause, NOW + 10_000).unwrap();
        let before = original.snapshot();
        let mut legacy = serde_json::to_value(&before).unwrap();
        let preferences = legacy["preferences"].as_object_mut().unwrap();
        preferences.remove("miniTimerEnabled");
        preferences.remove("miniTimerAlwaysOnTop");
        fs::write(dir.state_path(), serde_json::to_vec(&legacy).unwrap()).unwrap();

        let mut store = dir.store();
        let loaded = store.load(NOW + 20_000).unwrap();
        assert!(loaded.recovered_corrupt.is_none());
        let snapshot = loaded.engine.snapshot();
        assert_eq!(snapshot.timer.status, TimerStatus::Paused);
        assert_eq!(snapshot.timer.remaining_ms, before.timer.remaining_ms);
        assert_eq!(snapshot.timer.run_id, before.timer.run_id);
        assert_eq!(snapshot.history, before.history);
        assert!(!snapshot.preferences.mini_timer_enabled);
        assert!(snapshot.preferences.mini_timer_always_on_top);
        store
            .save(snapshot.revision, &loaded.engine.serialize().unwrap())
            .unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.state_path()).unwrap()).unwrap();
        assert_eq!(saved["version"], 1);
        assert_eq!(saved["preferences"]["miniTimerEnabled"], false);
        assert_eq!(saved["preferences"]["miniTimerAlwaysOnTop"], true);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[test]
    fn stale_or_duplicate_saves_never_overwrite_newer_state() {
        let dir = Fixture::new();
        let mut store = dir.store();
        let mut engine = store.load(NOW).unwrap().engine;
        engine.dispatch(TimerAction::Start, NOW).unwrap();
        let old_revision = engine.snapshot().revision;
        let old = engine.serialize().unwrap();
        engine.dispatch(TimerAction::Pause, NOW + 10_000).unwrap();
        let new_revision = engine.snapshot().revision;
        let new = engine.serialize().unwrap();
        store.save(new_revision, &new).unwrap();
        store.save(old_revision, &old).unwrap();
        store.save(new_revision, &new).unwrap();
        assert_eq!(fs::read_to_string(dir.state_path()).unwrap(), new);
    }

    #[test]
    fn corrupt_state_is_preserved_byte_for_byte_before_new_state_is_allowed() {
        let dir = Fixture::new();
        let damaged = b"{broken-json-\xff";
        fs::write(dir.state_path(), damaged).unwrap();
        let mut store = dir.store();
        let loaded = store.load(NOW).unwrap();
        let preserved = loaded.recovered_corrupt.unwrap();
        assert_eq!(fs::read(&preserved).unwrap(), damaged);
        assert!(!dir.state_path().exists());
        let engine = loaded.engine;
        store
            .save(engine.snapshot().revision, &engine.serialize().unwrap())
            .unwrap();
        assert_eq!(fs::read(preserved).unwrap(), damaged);
        assert!(
            serde_json::from_str::<Snapshot>(&fs::read_to_string(dir.state_path()).unwrap())
                .is_ok()
        );
    }

    #[test]
    fn oversized_file_is_preserved_without_reading_unbounded_data() {
        let dir = Fixture::new();
        let mut file = File::create(dir.state_path()).unwrap();
        file.set_len((MAX_STATE_BYTES + 1024) as u64).unwrap();
        file.write_all(b"oversized").unwrap();
        drop(file);
        let mut store = dir.store();
        let loaded = store.load(NOW).unwrap();
        let preserved = loaded.recovered_corrupt.unwrap();
        assert_eq!(
            fs::metadata(preserved).unwrap().len(),
            (MAX_STATE_BYTES + 1024) as u64
        );
    }

    #[test]
    fn unknown_newer_schema_blocks_writes_and_retains_primary_file() {
        let dir = Fixture::new();
        let raw = r#"{"version":2,"futureData":"keep this"}"#;
        fs::write(dir.state_path(), raw).unwrap();
        let mut store = dir.store();
        assert!(matches!(store.load(NOW), Err(message) if message == "unsupported_state_version"));
        let engine = Engine::new(NOW);
        assert_eq!(
            store
                .save(engine.snapshot().revision, &engine.serialize().unwrap())
                .unwrap_err(),
            "storage_write_blocked"
        );
        assert_eq!(fs::read_to_string(dir.state_path()).unwrap(), raw);
    }

    #[test]
    fn nonregular_state_path_blocks_writes_instead_of_removing_user_data() {
        let dir = Fixture::new();
        fs::create_dir(dir.state_path()).unwrap();
        fs::write(dir.state_path().join("keep.txt"), "keep").unwrap();
        let mut store = dir.store();
        assert!(store.load(NOW).is_err());
        let engine = Engine::new(NOW);
        assert!(store
            .save(engine.snapshot().revision, &engine.serialize().unwrap())
            .is_err());
        assert_eq!(
            fs::read_to_string(dir.state_path().join("keep.txt")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn invalid_write_leaves_last_good_state_and_no_partial_temp_files() {
        let dir = Fixture::new();
        let mut store = dir.store();
        let mut engine = store.load(NOW).unwrap().engine;
        let valid = engine.serialize().unwrap();
        store.save(engine.snapshot().revision, &valid).unwrap();
        assert!(store.save(1, "{bad").is_err());
        assert!(store.save(1, &valid).is_err());
        engine.dispatch(TimerAction::Start, NOW).unwrap();
        let mut value = serde_json::to_value(engine.snapshot()).unwrap();
        value["timer"]["remainingMs"] = (-1).into();
        assert!(store
            .save(engine.snapshot().revision, &value.to_string())
            .is_err());
        assert_eq!(fs::read_to_string(dir.state_path()).unwrap(), valid);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[test]
    fn valid_replacements_use_one_complete_file_and_restore_newest_revision() {
        let dir = Fixture::new();
        let mut store = dir.store();
        let mut engine = store.load(NOW).unwrap().engine;
        store.save(0, &engine.serialize().unwrap()).unwrap();
        for i in 0..5 {
            engine
                .dispatch(
                    TimerAction::SetIntent {
                        intent: format!("Round {i}"),
                    },
                    NOW + i,
                )
                .unwrap();
            store
                .save(engine.snapshot().revision, &engine.serialize().unwrap())
                .unwrap();
            assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
            let saved: Snapshot =
                serde_json::from_str(&fs::read_to_string(dir.state_path()).unwrap()).unwrap();
            assert_eq!(saved.timer.intent, format!("Round {i}"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(dir.state_path()).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn store_cannot_write_until_existing_data_has_been_checked() {
        let dir = Fixture::new();
        fs::write(dir.state_path(), b"valuable unread data").unwrap();
        let mut store = dir.store();
        let engine = Engine::new(NOW);
        assert_eq!(
            store.save(0, &engine.serialize().unwrap()).unwrap_err(),
            "storage_write_blocked"
        );
        assert_eq!(fs::read(dir.state_path()).unwrap(), b"valuable unread data");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_state_file_does_not_read_or_replace_unrelated_target() {
        use std::os::unix::fs::symlink;
        let dir = Fixture::new();
        let outside = Fixture::new();
        let target = outside.0.join("keep.json");
        fs::write(&target, b"keep external data").unwrap();
        symlink(&target, dir.state_path()).unwrap();
        let mut store = dir.store();
        assert!(store.load(NOW).is_err());
        let engine = Engine::new(NOW);
        assert!(store.save(0, &engine.serialize().unwrap()).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"keep external data");
        assert!(fs::symlink_metadata(dir.state_path())
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
