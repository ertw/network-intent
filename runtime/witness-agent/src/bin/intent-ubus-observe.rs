//! Private, single-request native libubus worker. Installed at a fixed path;
//! only its supervising parent provides deadlines and concurrency limits.
fn main() {
    if intent_witness_agent::adapters::native_ubus::helper_main().is_err() {
        std::process::exit(1);
    }
}
