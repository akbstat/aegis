// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

mod commands;
mod http;
mod state;
mod system;
mod trace_id_setup;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // auth
            commands::auth::login,
            commands::auth::login_domain,
            commands::auth::is_logged_in,
            commands::auth::refresh,
            commands::auth::logout,
            // identity
            commands::identity::get_domain_user_info,
            // user-credential
            commands::user_credential::register_user,
            commands::user_credential::update_user_credential,
            // user
            commands::user::create_user,
            commands::user::list_users,
            commands::user::get_user_by_code,
            commands::user::current_user,
            commands::user::update_user,
            // project
            commands::project::create_project,
            commands::project::list_projects,
            commands::project::get_project_by_code,
            commands::project::update_project,
            // terminology
            commands::terminology::version::create_terminology_version,
            commands::terminology::version::list_terminology_versions,
            commands::terminology::version::get_terminology_version_by_id,
            commands::terminology::version::update_terminology_version,
            commands::terminology::version::delete_terminology_version,
            commands::terminology::code_list::create_code_list,
            commands::terminology::code_list::list_code_lists,
            commands::terminology::code_list::get_code_list_by_id,
            commands::terminology::code_list::update_code_list,
            commands::terminology::code_list::delete_code_list,
            commands::terminology::code_item::create_code_item,
            commands::terminology::code_item::list_code_items,
            commands::terminology::code_item::update_code_item,
            commands::terminology::code_item::delete_code_item,
            commands::terminology::code_item::list_code_items_by_version_and_code,
            commands::terminology::import::import_terminology,
            // domain-model
            commands::domain_model::version::create_sdtm_version,
            commands::domain_model::version::list_sdtm_versions,
            commands::domain_model::version::get_sdtm_version_by_id,
            commands::domain_model::version::update_sdtm_version,
            commands::domain_model::version::delete_sdtm_version,
            commands::domain_model::domain::create_sdtm_domain,
            commands::domain_model::domain::list_sdtm_domains_by_version,
            commands::domain_model::domain::get_sdtm_domain_by_id,
            commands::domain_model::domain::update_sdtm_domain,
            commands::domain_model::domain::delete_sdtm_domain,
            commands::domain_model::variable::create_sdtm_variable,
            commands::domain_model::variable::list_sdtm_variables_by_domain,
            commands::domain_model::variable::get_sdtm_variable_by_id,
            commands::domain_model::variable::update_sdtm_variable,
            commands::domain_model::variable::delete_sdtm_variable,
            // mission
            commands::mission::list_missions_by_project,
            commands::mission::add_assignee,
            commands::mission::remove_assignee,
            commands::mission::create_mission,
            commands::mission::list_issues_by_mission,
            commands::mission::create_issue,
            commands::mission::patch_issue_state,
            commands::mission::update_issue_description,
            commands::mission::append_comment,
            // crf
            commands::crf::version::list_crf_versions,
            commands::crf::version::import_als,
            commands::crf::form::list_crf_forms_by_version,
            commands::crf::form::create_crf_form,
            commands::crf::form::update_crf_form,
            commands::crf::form::delete_crf_form,
            commands::crf::form::get_crf_form_by_id,
            commands::crf::form::get_crf_form_details,
            commands::crf::form::search_crf_forms_by_version,
            commands::crf::form::set_crf_form_approved,
            commands::crf::item::list_crf_items_by_form,
            commands::crf::item::get_crf_item_by_id,
            commands::crf::item::update_crf_item,
            commands::crf::item::search_crf_items_by_version,
            commands::crf::option::update_crf_option,
            commands::crf::option::get_crf_option_by_id,
            commands::crf::option::search_crf_options_by_version,
            commands::crf::unit::update_crf_unit,
            commands::crf::unit::get_crf_unit_by_id,
            commands::crf::unit::search_crf_units_by_version,
            commands::crf::annotation::create_crf_annotation,
            commands::crf::annotation::update_crf_annotation,
            commands::crf::annotation::delete_crf_annotation,
            commands::crf::annotation::search_crf_annotations_by_version,
            commands::crf::domain_annotation::create_crf_domain_annotation,
            commands::crf::domain_annotation::list_crf_domain_annotations_by_form,
            commands::crf::domain_annotation::update_crf_domain_annotation,
            commands::crf::domain_annotation::delete_crf_domain_annotation,
            commands::crf::domain_annotation::search_crf_domain_annotations_by_version,
            // structures
            commands::structures::list_structures,
            commands::structures::get_structure,
            commands::structures::create_structure,
            commands::structures::update_structure,
            commands::structures::delete_structure,
            commands::structures::add_structure_node,
            commands::structures::update_structure_node,
            commands::structures::delete_structure_node,
            // health
            commands::healthz::healthz,
            // webview log forwarder (sink for console.warn / console.error)
            commands::webview_log::forward_webview_log,
            // legacy greet (kept for the existing test)
            greet,
        ])
        .setup(|app| {
            // Order matters. The submitter needs the HTTP client, and
            // the submit layer has to be installed in the *same*
            // `init_tracing` call that installs the file layer —
            // `try_init` only takes effect once per process, so a
            // second call would silently add nothing. Building the
            // client first costs us only the logs emitted before
            // `setup` runs, which the previous ordering lost anyway.
            let store = app
                .store("auth.bin")
                .map_err(|e| format!("failed to open auth.bin store: {e}"))?;
            let tokens = Arc::new(http::client::TauriStore::new(store));
            let client = http::client::HttpClient::new(http::config::BASE_URL.to_string(), tokens);

            // Per-install device prefix: persisted in app-data dir so
            // a workstation keeps a stable middle segment on every
            // trace id it emits — and, now, on every batch id the
            // submitter mints. load_or_create is best-effort and
            // falls back to a freshly-minted, non-persisted prefix
            // on I/O errors.
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app_data_dir: {e}"))?;
            let generator = trace_id_setup::load_or_create(&app_data_dir);
            tracing::info!(
                device_prefix_file = %app_data_dir
                    .join(trace_id_setup::DEVICE_PREFIX_FILE_NAME)
                    .display(),
                "trace id generator ready"
            );
            // Read the prefix before the generator is moved into
            // managed state, so a batch id and a trace id from this
            // install always share the same middle segment. We clone
            // here so the legacy `app.manage(generator)` below continues
            // to back State<'_, TraceIdGenerator> lookups from
            // commands/auth.rs (and other legacy shims), and a second
            // copy of the generator — sharing the same device prefix —
            // also lives inside SharedAppState.
            let device_id = generator.device_prefix().unwrap_or("desktop").to_string();
            app.manage(generator.clone());

            // Batched submission of this process's own logs to the
            // server. A failed batch is persisted under
            // <app_data_dir>/pending-logs and retried newest-first
            // on later cycles, bounded by `max_pending_files`.
            let submitter = logging_utils::LogSubmitter::new(
                logging_utils::LogSubmitterConfig::builder()
                    .device_id(device_id)
                    .sender(Arc::new(http::log_ingest::HttpLogSender::new(Arc::new(
                        client.clone(),
                    ))))
                    .pending_dir(app_data_dir.join("pending-logs"))
                    // Must outlast a single submit: `HttpClient` has a
                    // 15s request timeout, so a shorter deadline
                    // detaches the worker mid-send on every shutdown
                    // that happens while the server is slow or down —
                    // exactly when draining matters most.
                    .shutdown_deadline(std::time::Duration::from_secs(20))
                    .build(),
            )
            .map_err(|e| format!("log submitter init: {e}"))?;

            // Tracing init: prefer $AEGIS_LOG_DIR; fall back to
            // <app_data_dir>/logs. LogGuard is stashed in managed
            // state so the buffered writer lives for the process.
            let log_dir = std::env::var("AEGIS_LOG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| app_data_dir.join("logs"));
            let log_guard = logging_utils::init_tracing(
                &logging_utils::LoggingConfig {
                    log_dir: log_dir.clone(),
                    file_name_prefix: "aegis-desktop.log".into(),
                },
                // The submit layer is formatted by the same
                // `fmt::Layer` configuration as the file layer, so a
                // shipped entry is byte-identical to the local line.
                vec![Box::new(logging_utils::submit_layer(submitter.handle()))],
            )
            .map_err(|e| format!("init_tracing: {e}"))?;
            tracing::info!(
                log_dir = %log_dir.display(),
                "aegis-desktop tracing initialised"
            );

            // Single aggregation point. The four legacy `app.manage(...)`
            // calls above continue to back the State<'_, HttpClient>
            // and State<'_, TraceIdGenerator> lookups from
            // commands/auth.rs, commands/user.rs,
            // commands/user_credential.rs, commands/identity.rs, and
            // commands/healthz.rs (those shims are still on the
            // pre-split shape). This is the fifth managed state entry
            // — the refactored shims in commands/{crf,domain_model,
            // mission,project,terminology}/* and a future LLM-agent
            // caller resolve against it.
            //
            // LogSubmitter and LogGuard are not Clone; they are MOVED
            // into SharedAppState here, which is why the lines
            // `app.manage(submitter);` and `app.manage(log_guard);`
            // that used to live above have been removed — their Drop
            // lifecycle is now identical (driven by SharedAppState's
            // own Drop, which Tauri invokes when the runtime tears
            // managed state down).
            let shared = state::SharedAppState::new(
                client.clone(), // HttpClient is Clone (Arc<reqwest::Client>)
                generator.clone(),
                submitter,
                log_guard,
            );
            app.manage(shared);

            // Legacy keep-alive for HttpClient so the out-of-scope
            // command shims keep resolving. Same lifetime guarantee
            // as before — managed for the Tauri runtime, dropped at
            // shutdown.
            app.manage(client);
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(|_app_handle, _event| {});
    Ok(())
}
