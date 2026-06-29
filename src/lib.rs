// Raised from the default 128: the `MergedObject` query root composes a field
// per enabled service, and the generated `resolve_field` recursion grows with
// the number of wired services. 1024 covers the full `--all-features` surface
// (the federation `find_entity` layout alone adds ~514 levels of depth).
#![recursion_limit = "1024"]

pub mod aws;
pub mod error;
pub mod schema;
pub mod server;
