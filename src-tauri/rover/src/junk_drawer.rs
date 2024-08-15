use std::time::SystemTime;

use chrono::{DateTime, Local};

pub fn system_time_to_string(system_time: SystemTime) -> String {
    let datetime: DateTime<Local> = system_time.into();
    // Format the datetime as a string, e.g. "2021-01-01 12:00:00"
    // The default datetime.to_string() call includes fractional seconds
    // and the timezone, which we don't want.
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
