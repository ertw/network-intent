//! Controller-local runtime state.  This crate deliberately performs no
//! signature authorization: transport authentication belongs at its boundary.
pub mod health;
pub mod admission;
pub mod ingress;
pub mod monitor;
pub mod storage;
