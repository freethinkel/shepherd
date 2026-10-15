mod client;
pub mod client_shell;
pub mod graphics;
mod config;
pub mod keys;
pub mod popup;
mod remote;
mod session;
mod wire;
pub use wire::Rgb;
pub use client::{Client, EVENT_TYPES};
pub use config::{HerdrConfig, config_path};
pub use session::{Mode, Session, SessionEvent, SessionInfo, current_session, delete_session, new_session, no_server, sessions, valid_session_name, set_binary, set_socket, start_server};
pub use wire::PaneInput;
pub use remote::{Machine, add_machine, git, locate, machines, quote, relay, remove_machine, rename_machine, show_machine, split, ssh};

/// Why herdr's server cannot carry Shepherd's panes, from its `ping`: it speaks another protocol than Shepherd. Set
/// by the sync loop, shown on every pane that loses its connection.
pub struct OutdatedServer(pub Option<String>);

impl gpui::Global for OutdatedServer {}

/// The complaint about a server whose `ping` answered `pong`; `None` when it speaks Shepherd's protocol (or does
/// not say).
pub fn protocol_mismatch(pong: &serde_json::Value) -> Option<String> {
    let protocol = pong["protocol"].as_u64()?;
    let ours = u64::from(wire::PROTOCOL_VERSION);
    let version = pong["version"].as_str().unwrap_or("unknown");
    if protocol == ours {
        None
    } else if protocol < ours {
        Some(format!(
            "herdr's server is {version} (protocol {protocol}); Shepherd needs protocol {ours}. Restart herdr's server so it runs the herdr you have installed."
        ))
    } else {
        Some(format!("herdr's server is {version} (protocol {protocol}), newer than Shepherd knows (protocol {ours}). Update Shepherd."))
    }
}

#[cfg(test)]
mod tests {
    use super::protocol_mismatch;
    use serde_json::json;

    #[test]
    fn a_server_on_another_protocol_is_named() {
        let ours = u64::from(super::wire::PROTOCOL_VERSION);
        assert_eq!(protocol_mismatch(&json!({"version": "x", "protocol": ours})), None);
        assert_eq!(protocol_mismatch(&json!({"version": "x"})), None, "a server that does not say");
        let old = protocol_mismatch(&json!({"version": "0.8.2", "protocol": ours - 2})).unwrap();
        assert!(old.contains("0.8.2") && old.contains("Restart"), "{old}");
        assert!(protocol_mismatch(&json!({"version": "9.0.0", "protocol": ours + 1})).unwrap().contains("Update Shepherd"));
    }
}
