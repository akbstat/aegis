//! Tauri command shims that delegate 1:1 to the `http` layer.
pub mod auth;
pub mod crf;
pub mod domain_model;
pub mod healthz;
pub mod identity;
pub mod mission;
pub mod project;
pub mod structures;
pub mod terminology;
pub mod user;
pub mod user_credential;
pub mod webview_log;

#[cfg(test)]
pub mod test_support;
