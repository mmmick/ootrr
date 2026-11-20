//! Small native calls into the operating system, kept out of the crates that
//! forbid unsafe code.

#[cfg(windows)]
pub mod pick;
#[cfg(not(windows))]
#[path = "nopick.rs"]
pub mod pick;
