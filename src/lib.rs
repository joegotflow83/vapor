// Raised from the default 128: the `MergedObject` query root composes a field
// per enabled service, and the generated `resolve_field` recursion grows with
// the number of wired services. 512 covers the full `--all-features` surface.
#![recursion_limit = "512"]

pub mod aws;
pub mod error;
pub mod schema;
pub mod server;
