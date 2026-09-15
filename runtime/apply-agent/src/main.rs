//! Separate apply-agent process. Guarded UCI execution and its durable journal
//! are deliberately not wired to this planning-only skeleton yet.
fn main() {
    eprintln!("intent-apply-agent: planning boundary only; no apply runtime configured");
}
