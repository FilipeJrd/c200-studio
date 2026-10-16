//! Linux camera backend; the only crate that touches device nodes and ioctls.
mod controls;
pub mod discovery;
pub mod preview;
pub mod vendor;
pub use controls::LinuxCamera;
