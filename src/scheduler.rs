use crate::settings::Settings;
use chrono::{DateTime, Duration, Local, NaiveTime, Timelike};
use std::time::{Duration as StdDuration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReminderState {
    Waiting,
    Paused,
    ActiveReminder,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub next_deadline: Option<Instant>,
    pub next_at_wall: Option<DateTime<Local>>,
    pub paused_until: Option<DateTime<Local>>,
    pub state: ReminderState,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            next_deadline: None,
            next_at_wall: None,
            paused_until: None,
            state: ReminderState::Waiting,
        }
    }
}

impl Schedule {
    pub fn schedule_next(&mut self, settings: &Settings, override_minutes: Option<u32>) {
        let minutes = override_minutes.unwrap_or(settings.interval_minutes);
        let interval = StdDuration::from_secs(u64::from(minutes) * 60);
        self.next_deadline = Some(Instant::now() + interval);
        self.next_at_wall = Some(Local::now() + Duration::seconds(interval.as_secs() as i64));
        self.state = ReminderState::Waiting;
    }

    pub fn pause_for_hours(&mut self, hours: i64) {
        self.paused_until = Some(Local::now() + Duration::hours(hours));
        self.next_deadline = None;
        self.next_at_wall = None;
        self.state = ReminderState::Paused;
    }

    pub fn pause_until_tomorrow(&mut self, settings: &Settings) {
        let now = Local::now();
        let start = start_time(settings);
        let tomorrow = now
            .date_naive()
            .succ_opt()
            .unwrap_or_else(|| now.date_naive());
        let target = tomorrow.and_time(start);
        self.paused_until = target.and_local_timezone(Local).single();
        self.next_deadline = None;
        self.next_at_wall = self.paused_until;
        self.state = ReminderState::Paused;
    }

    pub fn resume(&mut self, settings: &Settings) {
        self.paused_until = None;
        self.schedule_next(settings, None);
    }

    pub fn remaining(&self) -> Option<StdDuration> {
        self.next_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
    }

    pub fn due_after_suspend(&self) -> bool {
        self.next_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
            && self.state != ReminderState::ActiveReminder
    }
}

pub fn within_active_hours(settings: &Settings, now: DateTime<Local>) -> bool {
    let hour = now.hour();
    settings.active_start_hour <= hour && hour < settings.active_end_hour
}

pub fn seconds_until_active_window(settings: &Settings, now: DateTime<Local>) -> u64 {
    let start = start_time(settings);
    let today_start = now.date_naive().and_time(start);
    let target = if now.naive_local() < today_start {
        today_start
    } else {
        now.date_naive()
            .succ_opt()
            .unwrap_or_else(|| now.date_naive())
            .and_time(start)
    };
    let seconds = (target - now.naive_local()).num_seconds();
    u64::try_from(seconds.max(0)).unwrap_or(0)
}

fn start_time(settings: &Settings) -> NaiveTime {
    NaiveTime::from_hms_opt(settings.active_start_hour, 0, 0).unwrap_or(NaiveTime::MIN)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{LocalResult, TimeZone};

    fn at(hour: u32, minute: u32) -> DateTime<Local> {
        match Local.with_ymd_and_hms(2026, 5, 19, hour, minute, 0) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(dt, _) => dt,
            LocalResult::None => panic!("fecha local invalida"),
        }
    }

    #[test]
    fn active_hours_are_start_inclusive_end_exclusive() {
        let settings = Settings::default();

        assert!(within_active_hours(&settings, at(9, 0)));
        assert!(!within_active_hours(&settings, at(22, 0)));
    }

    #[test]
    fn seconds_until_active_window_uses_today_when_before_start() {
        let settings = Settings::default();

        assert_eq!(seconds_until_active_window(&settings, at(8, 30)), 30 * 60);
    }

    #[test]
    fn seconds_until_active_window_uses_tomorrow_after_end() {
        let settings = Settings::default();

        assert_eq!(
            seconds_until_active_window(&settings, at(22, 30)),
            10 * 60 * 60 + 30 * 60
        );
    }
}
