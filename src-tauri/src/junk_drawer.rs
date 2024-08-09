use std::time::SystemTime;

use chrono::{DateTime, Local};

pub fn system_time_to_string(system_time: SystemTime) -> String {
    let datetime: DateTime<Local> = system_time.into();
    datetime.to_string()
}