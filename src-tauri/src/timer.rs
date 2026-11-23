use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub const MIN_DURATION_MS: i64 = 60_000;
pub const MAX_DURATION_MS: i64 = 180 * MIN_DURATION_MS;
pub const CHECKPOINT_MS: i64 = 30_000;
pub const MAX_HISTORY: usize = 500;
pub const MAX_DAILY_STATS: usize = 366;
pub const MAX_STATE_BYTES: usize = 2 * 1024 * 1024;
const MAX_TIMESTAMP_MS: i64 = 253_402_214_400_000;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_DAILY_COUNT: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerMode {
    Focus,
    ShortBreak,
    LongBreak,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionOutcome {
    Completed,
    Ended,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Durations {
    pub focus: u16,
    pub short_break: u16,
    pub long_break: u16,
}
impl Durations {
    pub fn minutes(&self, mode: TimerMode) -> u16 {
        match mode {
            TimerMode::Focus => self.focus,
            TimerMode::ShortBreak => self.short_break,
            TimerMode::LongBreak => self.long_break,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MiniTimerStyle {
    #[default]
    Ring,
    Tomato,
    Capsule,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub durations: Durations,
    pub language: String,
    pub theme: String,
    pub notifications_enabled: bool,
    #[serde(default = "default_sound_enabled")]
    pub sound_enabled: bool,
    pub close_to_tray: bool,
    #[serde(default)]
    pub mini_timer_enabled: bool,
    #[serde(default = "default_mini_timer_always_on_top")]
    pub mini_timer_always_on_top: bool,
    #[serde(default)]
    pub mini_timer_style: MiniTimerStyle,
}
fn default_sound_enabled() -> bool {
    // Existing completion notifications used the system's default sound.
    true
}
fn default_mini_timer_always_on_top() -> bool {
    true
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            durations: Durations {
                focus: 25,
                short_break: 5,
                long_break: 15,
            },
            language: "system".into(),
            theme: "system".into(),
            notifications_enabled: false,
            sound_enabled: true,
            close_to_tray: true,
            mini_timer_enabled: false,
            mini_timer_always_on_top: true,
            mini_timer_style: MiniTimerStyle::Ring,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimerState {
    pub mode: TimerMode,
    pub status: TimerStatus,
    pub duration_ms: i64,
    pub remaining_ms: i64,
    pub deadline_at: Option<i64>,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub intent: String,
    pub run_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionRecord {
    pub id: String,
    pub mode: TimerMode,
    pub intent: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub duration_ms: i64,
    pub elapsed_ms: i64,
    pub outcome: SessionOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DailyStat {
    pub date: String,
    pub focus_ms: i64,
    pub completed_sessions: u32,
    pub ended_sessions: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub version: u8,
    pub revision: u64,
    pub timer: TimerState,
    pub preferences: Preferences,
    pub history: Vec<SessionRecord>,
    pub daily_stats: Vec<DailyStat>,
    pub cycle_completed: u8,
    pub persistence_error: bool,
    pub restored_paused: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum TimerAction {
    Start,
    Pause,
    Resume,
    Reset,
    End,
    SwitchMode { mode: TimerMode },
    SetDuration { minutes: u16 },
    SetIntent { intent: String },
}

#[derive(Clone, Debug)]
pub struct Transition {
    pub snapshot: Snapshot,
    pub changed: bool,
    pub persist: bool,
    pub completed: Option<SessionRecord>,
}

pub struct Engine {
    state: Snapshot,
    last_checkpoint_at: i64,
}

fn valid_time(value: i64) -> bool {
    (0..=MAX_TIMESTAMP_MS).contains(&value)
}
fn valid_minutes(value: u16) -> bool {
    (1..=180).contains(&value)
}
fn valid_duration(value: i64) -> bool {
    (MIN_DURATION_MS..=MAX_DURATION_MS).contains(&value) && value % MIN_DURATION_MS == 0
}
fn valid_intent(value: &str) -> bool {
    value.chars().count() <= 120 && !value.chars().any(char::is_control)
}
fn valid_id(value: &str) -> bool {
    value.len() == 36 && Uuid::parse_str(value).is_ok()
}
fn active(status: TimerStatus) -> bool {
    matches!(status, TimerStatus::Running | TimerStatus::Paused)
}
fn validate_preferences(value: &Preferences) -> Result<(), String> {
    if ![
        value.durations.focus,
        value.durations.short_break,
        value.durations.long_break,
    ]
    .iter()
    .all(|m| valid_minutes(*m))
        || !["system", "zh-CN", "zh-TW", "en", "ja", "ko"].contains(&value.language.as_str())
        || !["system", "light", "dark"].contains(&value.theme.as_str())
    {
        return Err("invalid_preferences".into());
    }
    Ok(())
}
fn local_day(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(timestamp)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .with_timezone(&Local)
        .format("%Y-%m-%d")
        .to_string()
}

pub(crate) fn validate_snapshot(value: &Snapshot) -> Result<(), String> {
    let invalid = || String::from("invalid_state");
    validate_preferences(&value.preferences).map_err(|_| invalid())?;
    let timer = &value.timer;
    if value.version != 1
        || value.revision >= MAX_SAFE_INTEGER
        || value.cycle_completed > 4
        || !valid_duration(timer.duration_ms)
        || !(0..=timer.duration_ms).contains(&timer.remaining_ms)
        || !valid_time(timer.updated_at)
        || !valid_intent(&timer.intent)
        || timer
            .started_at
            .is_some_and(|t| !valid_time(t) || t > timer.updated_at)
        || timer.deadline_at.is_some_and(|t| !valid_time(t))
        || timer.run_id.as_ref().is_some_and(|id| !valid_id(id))
        || (value.restored_paused && timer.status != TimerStatus::Paused)
    {
        return Err(invalid());
    }
    match timer.status {
        TimerStatus::Idle => {
            if timer.remaining_ms != timer.duration_ms
                || timer.deadline_at.is_some()
                || timer.started_at.is_some()
                || timer.run_id.is_some()
            {
                return Err(invalid());
            }
        }
        TimerStatus::Running => {
            if timer.remaining_ms == 0
                || timer.started_at.is_none()
                || timer.run_id.is_none()
                || timer
                    .deadline_at
                    .and_then(|d| d.checked_sub(timer.updated_at))
                    != Some(timer.remaining_ms)
            {
                return Err(invalid());
            }
        }
        TimerStatus::Paused => {
            if timer.remaining_ms == 0
                || timer.started_at.is_none()
                || timer.run_id.is_none()
                || timer.deadline_at.is_some()
            {
                return Err(invalid());
            }
        }
        TimerStatus::Completed => {
            if timer.remaining_ms != 0
                || timer.started_at.is_none()
                || timer.run_id.is_none()
                || timer.deadline_at.is_some()
            {
                return Err(invalid());
            }
        }
    }
    if value.history.len() > MAX_HISTORY || value.daily_stats.len() > MAX_DAILY_STATS {
        return Err(invalid());
    }
    let mut ids = HashSet::new();
    for record in &value.history {
        if !valid_id(&record.id)
            || !ids.insert(record.id.as_str())
            || !valid_intent(&record.intent)
            || !valid_time(record.started_at)
            || !valid_time(record.finished_at)
            || record.started_at > record.finished_at
            || !valid_duration(record.duration_ms)
            || record.elapsed_ms <= 0
            || record.elapsed_ms > record.duration_ms
            || (record.outcome == SessionOutcome::Completed
                && record.elapsed_ms != record.duration_ms)
            || (record.outcome == SessionOutcome::Ended
                && (record.mode != TimerMode::Focus || record.elapsed_ms == record.duration_ms))
        {
            return Err(invalid());
        }
    }
    if active(timer.status)
        && timer
            .run_id
            .as_ref()
            .is_some_and(|id| ids.contains(id.as_str()))
    {
        return Err(invalid());
    }
    let mut dates = HashSet::new();
    for stat in &value.daily_stats {
        let date = NaiveDate::parse_from_str(&stat.date, "%Y-%m-%d").map_err(|_| invalid())?;
        let count = u64::from(stat.completed_sessions) + u64::from(stat.ended_sessions);
        let minimum =
            i64::from(stat.completed_sessions) * MIN_DURATION_MS + i64::from(stat.ended_sessions);
        if stat.date.len() != 10
            || date.format("%Y-%m-%d").to_string() != stat.date
            || !dates.insert(stat.date.as_str())
            || stat.completed_sessions > MAX_DAILY_COUNT
            || stat.ended_sessions > MAX_DAILY_COUNT
            || count == 0
            || stat.focus_ms < minimum
            || stat.focus_ms > (count as i64) * MAX_DURATION_MS
        {
            return Err(invalid());
        }
    }
    Ok(())
}

impl Engine {
    pub fn new(now: i64) -> Self {
        let now = now.clamp(0, MAX_TIMESTAMP_MS - MAX_DURATION_MS);
        let preferences = Preferences::default();
        let duration_ms = i64::from(preferences.durations.focus) * MIN_DURATION_MS;
        Self {
            state: Snapshot {
                version: 1,
                revision: 0,
                timer: TimerState {
                    mode: TimerMode::Focus,
                    status: TimerStatus::Idle,
                    duration_ms,
                    remaining_ms: duration_ms,
                    deadline_at: None,
                    started_at: None,
                    updated_at: now,
                    intent: String::new(),
                    run_id: None,
                },
                preferences,
                history: Vec::new(),
                daily_stats: Vec::new(),
                cycle_completed: 0,
                persistence_error: false,
                restored_paused: false,
            },
            last_checkpoint_at: now,
        }
    }

    pub fn restore(raw: &str, now: i64) -> Result<Self, String> {
        if raw.len() > MAX_STATE_BYTES || !valid_time(now) {
            return Err("invalid_state".into());
        }
        let mut state: Snapshot =
            serde_json::from_str(raw).map_err(|_| String::from("invalid_state"))?;
        validate_snapshot(&state)?;
        // A saved running checkpoint is not proof that the app kept running while closed.
        let was_running = state.timer.status == TimerStatus::Running;
        if now < state.timer.updated_at {
            let adjustment = now - state.timer.updated_at;
            state.timer.started_at = state
                .timer
                .started_at
                .map(|time| (time + adjustment).max(0));
        }
        if was_running {
            state.timer.status = TimerStatus::Paused;
            state.timer.deadline_at = None;
        }
        state.timer.updated_at = now;
        state.restored_paused =
            was_running || (state.restored_paused && state.timer.status == TimerStatus::Paused);
        state.persistence_error = false;
        state.revision += 1;
        validate_snapshot(&state)?;
        Ok(Self {
            state,
            last_checkpoint_at: now,
        })
    }

    /// Android's same-boot service recovery keeps the authorized deadline.
    /// The caller must immediately tick using its persisted monotonic clock
    /// anchor; device reboots and unknown anchors use restore() instead.
    #[cfg(any(target_os = "android", test))]
    pub(crate) fn restore_continuing(raw: &str, now: i64) -> Result<Self, String> {
        if raw.len() > MAX_STATE_BYTES || !valid_time(now) {
            return Err("invalid_state".into());
        }
        let mut state: Snapshot =
            serde_json::from_str(raw).map_err(|_| String::from("invalid_state"))?;
        validate_snapshot(&state)?;
        state.persistence_error = false;
        state.revision += 1;
        validate_snapshot(&state)?;
        Ok(Self {
            state,
            last_checkpoint_at: now,
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }

    pub fn serialize(&self) -> Result<String, String> {
        validate_snapshot(&self.state)?;
        let raw =
            serde_json::to_string(&self.state).map_err(|_| String::from("serialization_failed"))?;
        if raw.len() > MAX_STATE_BYTES {
            return Err("state_too_large".into());
        }
        Ok(raw)
    }

    pub fn set_persistence_error(&mut self, value: bool) {
        if self.state.persistence_error != value {
            self.state.persistence_error = value;
            self.bump_revision();
        }
    }

    fn bump_revision(&mut self) {
        self.state.revision = self
            .state
            .revision
            .saturating_add(1)
            .min(MAX_SAFE_INTEGER - 1);
    }

    fn transition(
        &mut self,
        changed: bool,
        persist: bool,
        completed: Option<SessionRecord>,
        now: i64,
    ) -> Transition {
        if changed {
            self.bump_revision();
        }
        if persist {
            self.last_checkpoint_at = now;
        }
        Transition {
            snapshot: self.snapshot(),
            changed,
            persist,
            completed,
        }
    }

    fn idle_round(&mut self, now: i64) {
        let timer = &mut self.state.timer;
        timer.status = TimerStatus::Idle;
        timer.remaining_ms = timer.duration_ms;
        timer.started_at = None;
        timer.deadline_at = None;
        timer.run_id = None;
        timer.updated_at = now;
        self.state.restored_paused = false;
    }

    fn rebase_clock(&mut self, now: i64) {
        let timer = &mut self.state.timer;
        if now < timer.updated_at {
            let adjustment = now - timer.updated_at;
            timer.started_at = timer.started_at.map(|time| (time + adjustment).max(0));
            if timer.status == TimerStatus::Running {
                timer.deadline_at = Some(now + timer.remaining_ms);
            }
            timer.updated_at = now;
        }
    }

    fn record(&mut self, outcome: SessionOutcome, finished_at: i64) -> SessionRecord {
        let timer = &self.state.timer;
        let record = SessionRecord {
            id: timer.run_id.clone().expect("active run has an id"),
            mode: timer.mode,
            intent: if timer.mode == TimerMode::Focus {
                timer.intent.clone()
            } else {
                String::new()
            },
            started_at: timer.started_at.expect("active run has a start"),
            finished_at,
            duration_ms: timer.duration_ms,
            elapsed_ms: timer.duration_ms - timer.remaining_ms,
            outcome,
        };
        // The state machine also prevents a second finish; this guard protects record identity.
        if self
            .state
            .history
            .iter()
            .any(|existing| existing.id == record.id)
        {
            return record;
        }
        self.state.history.insert(0, record.clone());
        self.state.history.truncate(MAX_HISTORY);
        if record.mode == TimerMode::Focus {
            let date = local_day(finished_at);
            let index = self
                .state
                .daily_stats
                .iter()
                .position(|stat| stat.date == date);
            if index.is_none() {
                self.state.daily_stats.push(DailyStat {
                    date,
                    focus_ms: 0,
                    completed_sessions: 0,
                    ended_sessions: 0,
                });
            }
            let index = index.unwrap_or(self.state.daily_stats.len() - 1);
            let stat = &mut self.state.daily_stats[index];
            stat.focus_ms = stat.focus_ms.saturating_add(record.elapsed_ms);
            match outcome {
                SessionOutcome::Completed => {
                    stat.completed_sessions = stat.completed_sessions.saturating_add(1)
                }
                SessionOutcome::Ended => {
                    stat.ended_sessions = stat.ended_sessions.saturating_add(1)
                }
            }
            self.state.daily_stats.sort_by(|a, b| b.date.cmp(&a.date));
            self.state.daily_stats.truncate(MAX_DAILY_STATS);
            if outcome == SessionOutcome::Completed {
                self.state.cycle_completed = (self.state.cycle_completed + 1).min(4);
            }
        } else if record.mode == TimerMode::LongBreak && outcome == SessionOutcome::Completed {
            self.state.cycle_completed = 0;
        }
        record
    }

    fn reconcile(&mut self, now: i64) -> Option<SessionRecord> {
        if self.state.timer.status != TimerStatus::Running {
            return None;
        }
        self.rebase_clock(now);
        let deadline = self
            .state
            .timer
            .deadline_at
            .expect("running timer has a deadline");
        self.state.timer.remaining_ms = (deadline - now).max(0).min(self.state.timer.remaining_ms);
        self.state.timer.updated_at = now;
        if self.state.timer.remaining_ms != 0 {
            return None;
        }
        let record = self.record(SessionOutcome::Completed, deadline.min(now));
        self.state.timer.status = TimerStatus::Completed;
        self.state.timer.deadline_at = None;
        self.state.restored_paused = false;
        Some(record)
    }

    pub fn dispatch(&mut self, action: TimerAction, now: i64) -> Result<Transition, String> {
        if !valid_time(now) || now > MAX_TIMESTAMP_MS - MAX_DURATION_MS {
            return Err("invalid_timestamp".into());
        }
        match &action {
            TimerAction::SetDuration { minutes } if !valid_minutes(*minutes) => {
                return Err("invalid_duration".into())
            }
            TimerAction::SetIntent { intent } if !valid_intent(intent) => {
                return Err("invalid_intent".into())
            }
            _ => {}
        }
        let before = self.state.clone();
        let completed = self.reconcile(now);
        if active(self.state.timer.status)
            && matches!(
                action,
                TimerAction::Start
                    | TimerAction::SwitchMode { .. }
                    | TimerAction::SetDuration { .. }
            )
        {
            self.state = before;
            return Err("timer_active".into());
        }
        self.rebase_clock(now);
        match action {
            TimerAction::Start => {
                let timer = &mut self.state.timer;
                timer.status = TimerStatus::Running;
                timer.remaining_ms = timer.duration_ms;
                timer.deadline_at = Some(now + timer.duration_ms);
                timer.started_at = Some(now);
                timer.run_id = Some(Uuid::new_v4().to_string());
                timer.updated_at = now;
                self.state.restored_paused = false;
            }
            TimerAction::Pause if self.state.timer.status == TimerStatus::Running => {
                self.state.timer.status = TimerStatus::Paused;
                self.state.timer.deadline_at = None;
                self.state.timer.updated_at = now;
            }
            TimerAction::Resume if self.state.timer.status == TimerStatus::Paused => {
                self.state.timer.status = TimerStatus::Running;
                self.state.timer.deadline_at = Some(now + self.state.timer.remaining_ms);
                self.state.timer.updated_at = now;
                self.state.restored_paused = false;
            }
            TimerAction::Reset | TimerAction::End => {
                if active(self.state.timer.status)
                    && self.state.timer.mode == TimerMode::Focus
                    && self.state.timer.remaining_ms < self.state.timer.duration_ms
                {
                    self.record(SessionOutcome::Ended, now);
                }
                self.idle_round(now);
            }
            TimerAction::SwitchMode { mode } if mode != self.state.timer.mode => {
                self.state.timer.mode = mode;
                self.state.timer.duration_ms =
                    i64::from(self.state.preferences.durations.minutes(mode)) * MIN_DURATION_MS;
                self.idle_round(now);
            }
            TimerAction::SetDuration { minutes } => {
                self.state.timer.duration_ms = i64::from(minutes) * MIN_DURATION_MS;
                self.idle_round(now);
            }
            TimerAction::SetIntent { intent } => {
                self.state.timer.intent = intent;
                self.state.timer.updated_at = now;
            }
            _ => {}
        }
        let changed = self.state != before;
        Ok(self.transition(changed, changed, completed, now))
    }

    /// An explicit next-round tap follows the same cycle on every native surface.
    /// The caller persists/emits only the final transition while holding its operation lock.
    pub(crate) fn start_next_completed(
        &mut self,
        expected_run_id: &str,
        now: i64,
    ) -> Result<Transition, String> {
        if self.state.timer.status != TimerStatus::Completed
            || self.state.timer.run_id.as_deref() != Some(expected_run_id)
        {
            return Err("stale_widget_action".into());
        }
        let mode = match self.state.timer.mode {
            TimerMode::Focus if self.state.cycle_completed >= 4 => TimerMode::LongBreak,
            TimerMode::Focus => TimerMode::ShortBreak,
            TimerMode::ShortBreak | TimerMode::LongBreak => TimerMode::Focus,
        };
        // SwitchMode validates the timestamp before changing anything; Start uses
        // the same valid time and is guaranteed to see the newly idle round.
        self.dispatch(TimerAction::SwitchMode { mode }, now)?;
        self.dispatch(TimerAction::Start, now)
    }

    pub fn update_preferences(
        &mut self,
        preferences: Preferences,
        now: i64,
    ) -> Result<Transition, String> {
        validate_preferences(&preferences)?;
        if !valid_time(now) {
            return Err("invalid_timestamp".into());
        }
        let before = self.state.clone();
        let completed = self.reconcile(now);
        self.rebase_clock(now);
        let mode = self.state.timer.mode;
        let changed_default =
            self.state.preferences.durations.minutes(mode) != preferences.durations.minutes(mode);
        if changed_default && !active(self.state.timer.status) {
            self.state.timer.duration_ms =
                i64::from(preferences.durations.minutes(mode)) * MIN_DURATION_MS;
            self.state.timer.remaining_ms = if self.state.timer.status == TimerStatus::Completed {
                0
            } else {
                self.state.timer.duration_ms
            };
        }
        self.state.preferences = preferences;
        let changed = self.state != before;
        Ok(self.transition(changed, changed, completed, now))
    }

    pub fn clear_history(&mut self, now: i64) -> Transition {
        if !valid_time(now) {
            return self.transition(false, false, None, self.state.timer.updated_at);
        }
        let before = self.state.clone();
        let completed = self.reconcile(now);
        self.state.history.clear();
        self.state.daily_stats.clear();
        let changed = self.state != before;
        self.transition(changed, changed, completed, now)
    }

    pub fn tick(&mut self, now: i64) -> Transition {
        if !valid_time(now) {
            return self.transition(false, false, None, self.state.timer.updated_at);
        }
        let before = self.state.timer.clone();
        let backwards = now < before.updated_at && before.status == TimerStatus::Running;
        let completed = self.reconcile(now);
        let changed = before != self.state.timer;
        let checkpoint = self.state.timer.status == TimerStatus::Running
            && now.saturating_sub(self.last_checkpoint_at) >= CHECKPOINT_MS;
        let persist = completed.is_some() || backwards || checkpoint;
        self.transition(changed, persist, completed, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    const START: i64 = 1_800_000_000_000;
    fn one_minute() -> Engine {
        let mut engine = Engine::new(START);
        engine
            .dispatch(TimerAction::SetDuration { minutes: 1 }, START)
            .unwrap();
        engine
    }
    fn run(engine: &mut Engine, now: i64) {
        engine.dispatch(TimerAction::Start, now).unwrap();
    }

    #[test]
    fn defaults_and_json_match_frontend_contract() {
        let engine = Engine::new(START);
        let value = serde_json::to_value(engine.snapshot()).unwrap();
        assert_eq!(value["timer"]["mode"], "focus");
        assert_eq!(value["timer"]["status"], "idle");
        assert_eq!(value["timer"]["durationMs"], 1_500_000);
        assert_eq!(value["preferences"]["durations"]["shortBreak"], 5);
        assert_eq!(value["preferences"]["notificationsEnabled"], false);
        assert_eq!(value["preferences"]["soundEnabled"], true);
        assert_eq!(value["preferences"]["miniTimerStyle"], "ring");
        assert_eq!(value["preferences"]["miniTimerEnabled"], false);
        assert_eq!(value["preferences"]["miniTimerAlwaysOnTop"], true);
        assert!(value.get("dailyStats").is_some());
        assert!(value.get("cycleCompleted").is_some());
        assert!(value.get("persistenceError").is_some());
        assert!(value.get("restoredPaused").is_some());
        for (json, action) in [
            (
                r#"{"type":"switchMode","mode":"longBreak"}"#,
                TimerAction::SwitchMode {
                    mode: TimerMode::LongBreak,
                },
            ),
            (
                r#"{"type":"setDuration","minutes":50}"#,
                TimerAction::SetDuration { minutes: 50 },
            ),
            (
                r#"{"type":"setIntent","intent":"读一章"}"#,
                TimerAction::SetIntent {
                    intent: "读一章".into(),
                },
            ),
        ] {
            assert_eq!(serde_json::from_str::<TimerAction>(json).unwrap(), action);
        }
        assert_eq!(
            serde_json::to_value(TimerAction::Pause).unwrap(),
            serde_json::json!({"type":"pause"})
        );
    }

    #[test]
    fn legacy_v1_preferences_restore_with_mini_timer_defaults_and_keep_records() {
        let mut engine = one_minute();
        run(&mut engine, START);
        engine.tick(START + 60_000);
        run(&mut engine, START + 60_000);
        engine.tick(START + 80_000);
        let before = engine.snapshot();
        let mut legacy = serde_json::to_value(&before).unwrap();
        let preferences = legacy["preferences"].as_object_mut().unwrap();
        preferences.remove("miniTimerEnabled");
        preferences.remove("miniTimerAlwaysOnTop");
        preferences.remove("soundEnabled");
        preferences.remove("miniTimerStyle");

        let restored =
            Engine::restore(&serde_json::to_string(&legacy).unwrap(), START + 90_000).unwrap();
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.version, 1);
        assert!(!snapshot.preferences.mini_timer_enabled);
        assert!(snapshot.preferences.mini_timer_always_on_top);
        assert!(snapshot.preferences.sound_enabled);
        assert_eq!(snapshot.preferences.mini_timer_style, MiniTimerStyle::Ring);
        assert_eq!(snapshot.preferences, before.preferences);
        assert_eq!(snapshot.timer.status, TimerStatus::Paused);
        assert_eq!(snapshot.timer.remaining_ms, before.timer.remaining_ms);
        assert_eq!(snapshot.timer.run_id, before.timer.run_id);
        assert_eq!(snapshot.history, before.history);
        assert_eq!(snapshot.daily_stats, before.daily_stats);
        assert_eq!(snapshot.cycle_completed, before.cycle_completed);
        let saved: serde_json::Value =
            serde_json::from_str(&restored.serialize().unwrap()).unwrap();
        assert_eq!(saved["preferences"]["miniTimerEnabled"], false);
        assert_eq!(saved["preferences"]["miniTimerAlwaysOnTop"], true);

        legacy["preferences"]["miniTimerEnabled"] = true.into();
        legacy["preferences"]["miniTimerAlwaysOnTop"] = false.into();
        legacy["preferences"]["soundEnabled"] = false.into();
        legacy["preferences"]["miniTimerStyle"] = "capsule".into();
        let explicit = Engine::restore(&serde_json::to_string(&legacy).unwrap(), START + 90_000)
            .unwrap()
            .snapshot();
        assert!(explicit.preferences.mini_timer_enabled);
        assert!(!explicit.preferences.mini_timer_always_on_top);
        assert!(!explicit.preferences.sound_enabled);
        assert_eq!(
            explicit.preferences.mini_timer_style,
            MiniTimerStyle::Capsule
        );
    }

    #[test]
    fn sound_and_widget_style_preserve_the_current_run_and_reject_invalid_data() {
        let mut engine = one_minute();
        run(&mut engine, START);
        engine.tick(START + 7_000);
        let before = engine.snapshot();
        for style in [
            MiniTimerStyle::Tomato,
            MiniTimerStyle::Capsule,
            MiniTimerStyle::Ring,
        ] {
            let mut preferences = engine.snapshot().preferences;
            preferences.sound_enabled = !preferences.sound_enabled;
            preferences.mini_timer_style = style;
            let changed = engine
                .update_preferences(preferences, START + 7_000)
                .unwrap();
            assert!(changed.persist);
            assert!(changed.completed.is_none());
            assert_eq!(changed.snapshot.timer, before.timer);
            assert_eq!(changed.snapshot.history, before.history);
            assert_eq!(changed.snapshot.cycle_completed, before.cycle_completed);
        }
        let mut invalid = serde_json::to_value(engine.snapshot()).unwrap();
        invalid["preferences"]["miniTimerStyle"] = "unrecognized".into();
        assert!(Engine::restore(&invalid.to_string(), START + 7_000).is_err());
        invalid["preferences"]["miniTimerStyle"] = "ring".into();
        invalid["preferences"]["soundEnabled"] = "yes".into();
        assert!(Engine::restore(&invalid.to_string(), START + 7_000).is_err());
    }

    #[test]
    fn mini_timer_preferences_preserve_current_round_and_records_in_every_status() {
        for status in [
            TimerStatus::Idle,
            TimerStatus::Running,
            TimerStatus::Paused,
            TimerStatus::Completed,
        ] {
            let mut engine = one_minute();
            run(&mut engine, START);
            engine.tick(START + 60_000);
            let mut now = START + 60_000;
            match status {
                TimerStatus::Idle => {
                    engine.dispatch(TimerAction::Reset, now).unwrap();
                }
                TimerStatus::Running | TimerStatus::Paused => {
                    run(&mut engine, now);
                    now += 7_000;
                    engine.tick(now);
                    if status == TimerStatus::Paused {
                        engine.dispatch(TimerAction::Pause, now).unwrap();
                    }
                }
                TimerStatus::Completed => {}
            }
            let before = engine.snapshot();
            for (enabled, always_on_top) in [(true, false), (false, true)] {
                let mut preferences = engine.snapshot().preferences;
                preferences.mini_timer_enabled = enabled;
                preferences.mini_timer_always_on_top = always_on_top;
                let transition = engine.update_preferences(preferences, now).unwrap();
                assert!(transition.changed);
                assert!(transition.persist);
                assert!(transition.completed.is_none());
                assert_eq!(transition.snapshot.timer, before.timer);
                assert_eq!(transition.snapshot.history, before.history);
                assert_eq!(transition.snapshot.daily_stats, before.daily_stats);
                assert_eq!(transition.snapshot.cycle_completed, before.cycle_completed);
                assert_eq!(transition.snapshot.preferences.mini_timer_enabled, enabled);
                assert_eq!(
                    transition.snapshot.preferences.mini_timer_always_on_top,
                    always_on_top
                );
            }
        }
    }

    #[test]
    fn pause_and_resume_exclude_paused_time_and_keep_identity() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let id = engine.snapshot().timer.run_id;
        engine.dispatch(TimerAction::Pause, START + 12_345).unwrap();
        assert_eq!(engine.snapshot().timer.remaining_ms, 47_655);
        assert!(!engine.tick(START + 90_000).changed);
        engine
            .dispatch(TimerAction::Resume, START + 90_000)
            .unwrap();
        assert_eq!(engine.snapshot().timer.run_id, id);
        assert_eq!(engine.snapshot().timer.deadline_at, Some(START + 137_655));
        let result = engine.tick(START + 137_655);
        assert_eq!(result.completed.unwrap().elapsed_ms, 60_000);
        assert_eq!(result.snapshot.history.len(), 1);
    }

    #[test]
    fn delayed_tick_and_sleep_complete_only_current_round_once() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let result = engine.tick(START + 5 * 60 * 60_000);
        assert!(result.persist);
        assert_eq!(result.completed.unwrap().finished_at, START + 60_000);
        assert_eq!(result.snapshot.timer.status, TimerStatus::Completed);
        assert_eq!(result.snapshot.history.len(), 1);
        assert_eq!(result.snapshot.cycle_completed, 1);
        for delta in [5 * 60 * 60_000, 6 * 60 * 60_000] {
            let duplicate = engine.tick(START + delta);
            assert!(!duplicate.changed);
            assert!(duplicate.completed.is_none());
        }
    }

    #[test]
    fn backward_clock_preserves_remainder_then_runs_from_new_deadline() {
        let mut engine = one_minute();
        run(&mut engine, START);
        engine.tick(START + 20_000);
        let rewind = engine.tick(START - 40_000);
        assert_eq!(rewind.snapshot.timer.remaining_ms, 40_000);
        assert_eq!(rewind.snapshot.timer.deadline_at, Some(START));
        assert_eq!(rewind.snapshot.timer.started_at, Some(START - 60_000));
        assert!(rewind.persist);
        assert_eq!(
            engine.tick(START - 30_000).snapshot.timer.remaining_ms,
            30_000
        );
        assert_eq!(engine.tick(START).completed.unwrap().elapsed_ms, 60_000);
        assert!(engine.serialize().is_ok());
    }

    #[test]
    fn reset_and_end_record_partial_focus_once_without_completed_credit() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let result = engine.dispatch(TimerAction::Reset, START + 17_001).unwrap();
        assert_eq!(result.snapshot.history[0].outcome, SessionOutcome::Ended);
        assert_eq!(result.snapshot.history[0].elapsed_ms, 17_001);
        assert_eq!(result.snapshot.daily_stats[0].focus_ms, 17_001);
        assert_eq!(result.snapshot.daily_stats[0].ended_sessions, 1);
        assert_eq!(result.snapshot.daily_stats[0].completed_sessions, 0);
        assert_eq!(result.snapshot.cycle_completed, 0);
        engine.dispatch(TimerAction::End, START + 18_000).unwrap();
        assert_eq!(engine.snapshot().history.len(), 1);
        assert_eq!(engine.snapshot().timer.remaining_ms, 60_000);
    }

    #[test]
    fn exact_expiry_before_pause_end_or_reset_is_completed_not_ended() {
        for action in [TimerAction::Pause, TimerAction::End, TimerAction::Reset] {
            let mut engine = one_minute();
            run(&mut engine, START);
            let result = engine.dispatch(action, START + 60_000).unwrap();
            assert_eq!(result.completed.unwrap().outcome, SessionOutcome::Completed);
            assert_eq!(result.snapshot.history.len(), 1);
            assert_eq!(result.snapshot.daily_stats[0].completed_sessions, 1);
            assert_eq!(result.snapshot.daily_stats[0].ended_sessions, 0);
        }
    }

    #[test]
    fn active_mode_and_duration_changes_are_rejected_without_mutation() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let before = engine.snapshot();
        for action in [
            TimerAction::SwitchMode {
                mode: TimerMode::ShortBreak,
            },
            TimerAction::SetDuration { minutes: 50 },
            TimerAction::Start,
        ] {
            assert_eq!(
                engine.dispatch(action, START + 10_000).unwrap_err(),
                "timer_active"
            );
            assert_eq!(engine.snapshot(), before);
        }
        engine.dispatch(TimerAction::Pause, START + 10_000).unwrap();
        assert_eq!(
            engine
                .dispatch(TimerAction::SetDuration { minutes: 2 }, START + 10_000)
                .unwrap_err(),
            "timer_active"
        );
    }

    #[test]
    fn language_theme_and_preferences_preserve_custom_round_and_active_timing() {
        let mut engine = one_minute();
        let mut prefs = engine.snapshot().preferences;
        prefs.language = "ja".into();
        prefs.theme = "dark".into();
        engine.update_preferences(prefs.clone(), START).unwrap();
        assert_eq!(engine.snapshot().timer.duration_ms, 60_000);
        run(&mut engine, START);
        prefs.durations.focus = 50;
        engine
            .update_preferences(prefs.clone(), START + 10_000)
            .unwrap();
        assert_eq!(engine.snapshot().timer.duration_ms, 60_000);
        assert_eq!(engine.snapshot().timer.remaining_ms, 50_000);
        engine.tick(START + 60_000);
        prefs.durations.focus = 30;
        engine.update_preferences(prefs, START + 60_001).unwrap();
        assert_eq!(engine.snapshot().timer.status, TimerStatus::Completed);
        assert_eq!(engine.snapshot().timer.remaining_ms, 0);
        assert_eq!(engine.snapshot().timer.duration_ms, 30 * 60_000);
        run(&mut engine, START + 60_002);
        assert_eq!(engine.snapshot().timer.duration_ms, 30 * 60_000);
    }

    #[test]
    fn checkpoint_is_throttled_and_restart_restores_checkpoint_as_paused() {
        let mut engine = one_minute();
        run(&mut engine, START);
        assert!(!engine.tick(START + 29_999).persist);
        let checkpoint = engine.tick(START + 30_000);
        assert!(checkpoint.persist);
        let raw = engine.serialize().unwrap();
        assert!(!engine.tick(START + 30_001).persist);
        let restored = Engine::restore(&raw, START + 24 * 60 * 60_000).unwrap();
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.timer.status, TimerStatus::Paused);
        assert_eq!(snapshot.timer.remaining_ms, 30_000);
        assert!(snapshot.restored_paused);
        assert!(snapshot.history.is_empty());
        assert_eq!(snapshot.timer.deadline_at, None);
    }

    #[test]
    fn completion_counts_and_long_break_cycle_reset_are_truthful() {
        let mut engine = one_minute();
        for i in 0..6 {
            run(&mut engine, START + i * 60_000);
            engine.tick(START + (i + 1) * 60_000);
        }
        assert_eq!(engine.snapshot().cycle_completed, 4);
        let focus_ms = engine
            .snapshot()
            .daily_stats
            .iter()
            .map(|s| s.focus_ms)
            .sum::<i64>();
        engine
            .dispatch(
                TimerAction::SwitchMode {
                    mode: TimerMode::ShortBreak,
                },
                START + 400_000,
            )
            .unwrap();
        run(&mut engine, START + 400_000);
        engine.dispatch(TimerAction::End, START + 410_000).unwrap();
        assert_eq!(engine.snapshot().history.len(), 6);
        engine
            .dispatch(
                TimerAction::SwitchMode {
                    mode: TimerMode::LongBreak,
                },
                START + 420_000,
            )
            .unwrap();
        engine
            .dispatch(TimerAction::SetDuration { minutes: 1 }, START + 420_000)
            .unwrap();
        run(&mut engine, START + 420_000);
        let result = engine.tick(START + 480_000);
        assert_eq!(result.completed.unwrap().mode, TimerMode::LongBreak);
        assert_eq!(result.snapshot.cycle_completed, 0);
        assert_eq!(result.snapshot.history.len(), 7);
        assert_eq!(
            result
                .snapshot
                .daily_stats
                .iter()
                .map(|s| s.focus_ms)
                .sum::<i64>(),
            focus_ms
        );
    }

    #[test]
    fn local_day_uses_completion_deadline_and_manual_ending_timestamp() {
        let start = Local
            .with_ymd_and_hms(2026, 6, 16, 23, 59, 30)
            .single()
            .unwrap()
            .timestamp_millis();
        let mut engine = one_minute();
        run(&mut engine, start);
        engine.tick(start + 8 * 60 * 60_000);
        assert_eq!(engine.snapshot().daily_stats[0].date, "2026-06-17");
        run(&mut engine, start + 9 * 60 * 60_000);
        engine
            .dispatch(TimerAction::End, start + 9 * 60 * 60_000 + 1_000)
            .unwrap();
        assert_eq!(engine.snapshot().daily_stats[0].ended_sessions, 1);
    }

    #[test]
    fn bounded_history_and_daily_totals_keep_newest_records() {
        let mut engine = one_minute();
        for i in 0..510 {
            let now = START + i * 86_400_000;
            run(&mut engine, now);
            engine.tick(now + 60_000);
        }
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.history.len(), MAX_HISTORY);
        assert_eq!(snapshot.daily_stats.len(), MAX_DAILY_STATS);
        assert!(snapshot.history[0].finished_at > snapshot.history[499].finished_at);
        assert!(snapshot.daily_stats[0].date > snapshot.daily_stats[365].date);
        assert!(Engine::restore(&engine.serialize().unwrap(), START + 511 * 86_400_000).is_ok());
    }

    #[test]
    fn clear_history_keeps_current_round_and_cycle_and_does_not_readd_completed() {
        let mut engine = one_minute();
        run(&mut engine, START);
        engine.tick(START + 60_000);
        let before = engine.snapshot().timer;
        let result = engine.clear_history(START + 60_000);
        assert_eq!(result.snapshot.timer, before);
        assert_eq!(result.snapshot.cycle_completed, 1);
        assert!(result.snapshot.history.is_empty());
        assert!(result.snapshot.daily_stats.is_empty());
        assert!(engine.tick(START + 70_000).snapshot.history.is_empty());
    }

    #[test]
    fn unicode_intent_and_invalid_inputs_are_bounded() {
        let mut engine = one_minute();
        engine
            .dispatch(
                TimerAction::SetIntent {
                    intent: "🍊".repeat(120),
                },
                START,
            )
            .unwrap();
        assert!(engine
            .dispatch(
                TimerAction::SetIntent {
                    intent: "🍊".repeat(121)
                },
                START
            )
            .is_err());
        assert!(engine
            .dispatch(
                TimerAction::SetIntent {
                    intent: "bad\nintent".into()
                },
                START
            )
            .is_err());
        for minutes in [0, 181] {
            assert_eq!(
                engine
                    .dispatch(TimerAction::SetDuration { minutes }, START)
                    .unwrap_err(),
                "invalid_duration"
            );
        }
        let mut preferences = engine.snapshot().preferences;
        preferences.language = "unsupported".into();
        assert_eq!(
            engine.update_preferences(preferences, START).unwrap_err(),
            "invalid_preferences"
        );
    }

    #[test]
    fn invalid_requests_at_deadline_leave_completion_available_to_next_tick() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let before = engine.snapshot();
        let deadline = START + 60_000;
        for action in [
            TimerAction::SetDuration { minutes: 0 },
            TimerAction::SetIntent {
                intent: "x".repeat(121),
            },
        ] {
            assert!(engine.dispatch(action, deadline).is_err());
            assert_eq!(engine.snapshot(), before);
        }
        let mut invalid_preferences = before.preferences.clone();
        invalid_preferences.theme = "invalid".into();
        assert!(engine
            .update_preferences(invalid_preferences, deadline)
            .is_err());
        assert_eq!(engine.snapshot(), before);
        let completion = engine.tick(deadline);
        assert!(completion.persist);
        assert!(completion.completed.is_some());
        assert_eq!(completion.snapshot.history.len(), 1);
        assert_eq!(completion.snapshot.daily_stats[0].completed_sessions, 1);
        assert!(engine.tick(deadline + 1).completed.is_none());
    }

    #[test]
    fn malformed_or_inconsistent_state_is_rejected_without_partial_restore() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let valid = serde_json::to_value(engine.snapshot()).unwrap();
        let mut cases = Vec::new();
        let mut bad = valid.clone();
        bad["version"] = 2.into();
        cases.push(bad);
        let mut bad = valid.clone();
        bad["timer"]["remainingMs"] = (-1).into();
        cases.push(bad);
        let mut bad = valid.clone();
        bad["timer"]["deadlineAt"] = (START + 999_999).into();
        cases.push(bad);
        let mut bad = valid.clone();
        bad["timer"]["runId"] = "not-an-id".into();
        cases.push(bad);
        let mut bad = valid.clone();
        bad["preferences"]["durations"]["focus"] = 181.into();
        cases.push(bad);
        let mut bad = valid.clone();
        bad["timer"]["intent"] = "x".repeat(121).into();
        cases.push(bad);
        let mut bad = valid.clone();
        bad["extra"] = true.into();
        cases.push(bad);
        for bad in cases {
            assert!(Engine::restore(&serde_json::to_string(&bad).unwrap(), START).is_err());
        }
        assert!(Engine::restore("{broken", START).is_err());
        assert!(Engine::restore(&"x".repeat(MAX_STATE_BYTES + 1), START).is_err());
    }

    #[test]
    fn widget_next_round_matches_main_ui_cycle_and_preserves_history() {
        let mut engine = one_minute();
        run(&mut engine, START);
        engine.tick(START + 60_000);
        let finished = engine.snapshot();
        let id = finished.timer.run_id.clone().unwrap();
        let next = engine.start_next_completed(&id, START + 61_000).unwrap();
        assert_eq!(next.snapshot.timer.mode, TimerMode::ShortBreak);
        assert_eq!(next.snapshot.timer.status, TimerStatus::Running);
        assert_eq!(next.snapshot.timer.remaining_ms, 5 * MIN_DURATION_MS);
        assert_eq!(next.snapshot.history, finished.history);
        assert_eq!(next.snapshot.cycle_completed, 1);
        assert!(next.persist && next.changed && next.completed.is_none());
        assert_ne!(next.snapshot.timer.run_id.as_deref(), Some(id.as_str()));

        engine.tick(START + 361_000);
        let break_id = engine.snapshot().timer.run_id.clone().unwrap();
        let focus = engine
            .start_next_completed(&break_id, START + 362_000)
            .unwrap();
        assert_eq!(focus.snapshot.timer.mode, TimerMode::Focus);
        assert_eq!(focus.snapshot.timer.remaining_ms, 25 * MIN_DURATION_MS);
    }

    #[test]
    fn fourth_completed_focus_offers_long_break_without_automatic_start() {
        let mut engine = one_minute();
        let mut now = START;
        for index in 0..4 {
            engine
                .dispatch(TimerAction::SetDuration { minutes: 1 }, now)
                .unwrap();
            run(&mut engine, now);
            now += MIN_DURATION_MS;
            engine.tick(now);
            assert_eq!(engine.snapshot().timer.status, TimerStatus::Completed);
            assert_eq!(engine.snapshot().cycle_completed, index + 1);
            now += 1_000;
        }
        let id = engine.snapshot().timer.run_id.clone().unwrap();
        let next = engine.start_next_completed(&id, now).unwrap();
        assert_eq!(next.snapshot.timer.mode, TimerMode::LongBreak);
        assert_eq!(next.snapshot.timer.duration_ms, 15 * MIN_DURATION_MS);
        engine.tick(now + 15 * MIN_DURATION_MS);
        assert_eq!(engine.snapshot().cycle_completed, 0);
        let id = engine.snapshot().timer.run_id.clone().unwrap();
        let focus = engine
            .start_next_completed(&id, now + 15 * MIN_DURATION_MS + 1)
            .unwrap();
        assert_eq!(focus.snapshot.timer.mode, TimerMode::Focus);
    }

    #[test]
    fn stale_or_invalid_widget_next_cannot_mutate_the_current_round() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let active = engine.snapshot();
        let id = active.timer.run_id.clone().unwrap();
        assert!(engine.start_next_completed(&id, START + 1_000).is_err());
        assert_eq!(engine.snapshot(), active);
        engine.tick(START + MIN_DURATION_MS);
        let complete = engine.snapshot();
        assert!(engine
            .start_next_completed("old-id", START + MIN_DURATION_MS)
            .is_err());
        assert!(engine.start_next_completed(&id, -1).is_err());
        assert_eq!(engine.snapshot(), complete);
        engine
            .start_next_completed(&id, START + MIN_DURATION_MS)
            .unwrap();
        let next = engine.snapshot();
        assert!(engine
            .start_next_completed(&id, START + MIN_DURATION_MS + 1)
            .is_err());
        assert_eq!(engine.snapshot(), next);
    }

    #[test]
    fn persistence_status_changes_do_not_change_timer() {
        let mut engine = one_minute();
        run(&mut engine, START);
        let before = engine.snapshot();
        engine.set_persistence_error(true);
        assert_eq!(engine.snapshot().timer, before.timer);
        assert!(engine.snapshot().revision > before.revision);
        let revision = engine.snapshot().revision;
        engine.set_persistence_error(true);
        assert_eq!(engine.snapshot().revision, revision);
        engine.set_persistence_error(false);
        assert!(!engine.snapshot().persistence_error);
    }
}
