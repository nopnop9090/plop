use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Build counter: increments on every actual rebuild (cargo re-runs the
    // build script whenever any package file changes).
    let counter: u64 = fs::read_to_string("build_info.txt")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
        + 1;
    let _ = fs::write("build_info.txt", counter.to_string());

    let (y, m, d) = civil_from_days(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 86400,
    );
    let date_code = format!("{:02}{:02}{:02}", y % 100, m, d);

    println!("cargo:rustc-env=PLOP_VERSION=1.1");
    println!("cargo:rustc-env=PLOP_MAJOR=1");
    println!("cargo:rustc-env=PLOP_MINOR=1");
    println!("cargo:rustc-env=PLOP_DATE_CODE={date_code}");
    println!("cargo:rustc-env=PLOP_BUILD_NO={counter}");
}

/// Days-since-epoch -> (year, month, day), Howard Hinnant's civil algorithm.
fn civil_from_days(z: u64) -> (i64, u32, u32) {
    let z = z as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
