use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use crate::request::RequestFields;

#[derive(Clone, Debug, Default)]
pub struct ProgressSnapshot {
    pub total_steps: usize,
    pub completed_steps: usize,
    pub successful_steps: usize,
    pub unavailable_steps: usize,
    pub individual_rows: usize,
    pub team_rows: usize,
    pub current_label: String,
    pub last_message: String,
    pub districts_with_data: BTreeSet<u8>,
    pub regions_with_data: BTreeSet<u8>,
    pub conferences_with_data: BTreeSet<u8>,
    pub years_with_data: BTreeSet<u16>,
    pub state_has_data: bool,
    pub cancel_requested: bool,
}

static ACTIVE_PROGRESS: OnceLock<Mutex<Option<Arc<Mutex<ProgressSnapshot>>>>> = OnceLock::new();

static CANCEL_REQUESTED: OnceLock<AtomicBool> = OnceLock::new();

fn cancel_flag() -> &'static AtomicBool {
    CANCEL_REQUESTED.get_or_init(|| AtomicBool::new(false))
}

fn holder() -> &'static Mutex<Option<Arc<Mutex<ProgressSnapshot>>>> {
    ACTIVE_PROGRESS.get_or_init(|| Mutex::new(None))
}

fn active_snapshot() -> Option<Arc<Mutex<ProgressSnapshot>>> {
    holder().lock().ok()?.clone()
}

pub struct ProgressGuard;

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = holder().lock() {
            *slot = None;
        }
    }
}

pub fn install(snapshot: Arc<Mutex<ProgressSnapshot>>) -> ProgressGuard {
    cancel_flag().store(false, Ordering::Relaxed);
    if let Ok(mut data) = snapshot.lock() {
        data.cancel_requested = false;
    }
    if let Ok(mut slot) = holder().lock() {
        *slot = Some(snapshot);
    }
    ProgressGuard
}

fn update<F>(f: F)
where
    F: FnOnce(&mut ProgressSnapshot),
{
    let Some(shared) = active_snapshot() else {
        return;
    };
    let Ok(mut snapshot) = shared.lock() else {
        return;
    };
    f(&mut snapshot);
}

pub fn add_total_steps(count: usize) {
    update(|snapshot| {
        snapshot.total_steps = snapshot.total_steps.saturating_add(count);
    });
}

pub fn set_message(message: impl Into<String>) {
    let message = message.into();
    update(|snapshot| {
        snapshot.last_message = message.clone();
    });
}

pub fn set_current_label(label: impl Into<String>) {
    let label = label.into();
    update(|snapshot| {
        snapshot.current_label = label.clone();
        snapshot.last_message = label;
    });
}

pub fn record_attempt(fields: &RequestFields, individual_rows: usize, team_rows: usize) {
    let had_data = individual_rows > 0 || team_rows > 0;
    update(|snapshot| {
        snapshot.completed_steps = snapshot.completed_steps.saturating_add(1);
        snapshot.individual_rows = snapshot.individual_rows.saturating_add(individual_rows);
        snapshot.team_rows = snapshot.team_rows.saturating_add(team_rows);

        let level = if fields.state {
            String::from("State")
        } else if let Some(region) = fields.region {
            format!("Region {region}")
        } else if let Some(district) = fields.district {
            format!("District {district}")
        } else {
            String::from("Unknown level")
        };

        if had_data {
            snapshot.successful_steps = snapshot.successful_steps.saturating_add(1);
            snapshot.conferences_with_data.insert(fields.conference);
            snapshot.years_with_data.insert(fields.year);
            if fields.state {
                snapshot.state_has_data = true;
            }
            if let Some(district) = fields.district {
                snapshot.districts_with_data.insert(district);
            }
            if let Some(region) = fields.region {
                snapshot.regions_with_data.insert(region);
            }
            snapshot.current_label = format!(
                "Found data for {} {}A {} ({individual_rows} individual, {team_rows} team)",
                fields.year, fields.conference, level
            );
            snapshot.last_message = snapshot.current_label.clone();
        } else {
            snapshot.unavailable_steps = snapshot.unavailable_steps.saturating_add(1);
            snapshot.current_label = format!(
                "No data for {} {}A {}",
                fields.year, fields.conference, level
            );
            snapshot.last_message = snapshot.current_label.clone();
        }
    });
}

pub fn mark_finished() {
    let cancelled = is_cancelled();
    update(|snapshot| {
        snapshot.cancel_requested = cancelled;
        if cancelled {
            snapshot.current_label = String::from("Scrape cancelled.");
            snapshot.last_message = String::from("Scrape cancelled.");
        } else {
            snapshot.last_message = String::from("Scrape complete.");
        }
    });
}


pub fn request_cancel() {
    cancel_flag().store(true, Ordering::Relaxed);
    update(|snapshot| {
        snapshot.cancel_requested = true;
        snapshot.current_label = String::from("Cancelling scrape...");
        snapshot.last_message = String::from("Cancellation requested. Finishing in-flight work...");
    });
}

pub fn is_cancelled() -> bool {
    cancel_flag().load(Ordering::Relaxed)
}
