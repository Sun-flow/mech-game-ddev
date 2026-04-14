use std::fs;

use log::LevelFilter;
use simplelog::{
    CombinedLogger, Config, TermLogger, TerminalMode, ColorChoice, WriteLogger,
};

/// Initialize logging. In debug builds, logs to both terminal (warn+) and a
/// timestamped file in `outputs/` (debug+). In release builds, `log` macros
/// are compiled to no-ops via the `release_max_level_off` feature.
pub fn init() {
    let _ = fs::create_dir_all("outputs");

    let timestamp = chrono_lite_timestamp();
    let instance_name = std::env::var("MECH_LOG_NAME").unwrap_or_else(|_| {
        format!("{}", std::process::id())
    });
    let log_path = format!("outputs/game-{}-{}.log", timestamp, instance_name);

    let file = fs::File::create(&log_path).expect("Failed to create log file");

    CombinedLogger::init(vec![
        // Terminal: warn and above (desync warnings, errors)
        TermLogger::new(
            LevelFilter::Warn,
            Config::default(),
            TerminalMode::Stderr,
            ColorChoice::Auto,
        ),
        // File: debug and above (everything)
        WriteLogger::new(LevelFilter::Debug, Config::default(), file),
    ])
    .expect("Failed to initialize logger");

    log::info!("Logging initialized: {}", log_path);
}

/// Simple timestamp without pulling in chrono: YYYY-MM-DD-HHMMSS
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;

    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to rough UTC components (no leap second handling, fine for filenames)
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to Y-M-D (simplified Gregorian)
    let mut y = 1970i32;
    let mut remaining_days = days as i32;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining_days < md {
            m = i;
            break;
        }
        remaining_days -= md;
    }

    format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        y,
        m + 1,
        remaining_days + 1,
        hours,
        minutes,
        seconds
    )
}

fn is_leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
