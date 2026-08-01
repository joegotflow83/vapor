//! Emits the list of AWS service features this binary was compiled with, so
//! `vapor services` can report them without a hand-maintained list.
//!
//! Cargo sets `CARGO_FEATURE_<NAME>` for every enabled feature while the build
//! script runs, which makes this list impossible to drift from the actual build
//! — unlike the `docs` feature group in Cargo.toml, which must be updated by
//! hand whenever a service is added.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

/// Feature names that are groupings rather than AWS services. These expand to
/// service features, so reporting them too would be noise.
const META_FEATURES: &[&str] = &[
    "default",
    "docs",
    "basic",
    "web",
    "data",
    "monitoring",
    "devops",
    "release",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let services: BTreeSet<String> = env::vars()
        .filter_map(|(key, _)| key.strip_prefix("CARGO_FEATURE_").map(str::to_lowercase))
        .filter(|name| !META_FEATURES.contains(&name.as_str()))
        .collect();

    let entries = services
        .iter()
        .map(|s| format!("    \"{s}\","))
        .collect::<Vec<_>>()
        .join("\n");

    let generated = format!("pub const ENABLED_SERVICES: &[&str] = &[\n{entries}\n];\n");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    fs::write(Path::new(&out_dir).join("enabled_services.rs"), generated)
        .expect("write enabled_services.rs");
}
