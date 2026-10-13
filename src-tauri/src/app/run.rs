use super::*;

mod setup;
mod handlers;

use setup::*;
use handlers::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = resolve_data_dir();
    // One copy per data folder — see `core::instance`. Checked before anything is written or
    // started: a second copy that got as far as polling Telegram has already done the harm.
    let wakes = match core::instance::claim(&data_dir) {
        core::instance::Claim::Running => {
            eprintln!("[superuser] already running on {} — brought that copy forward instead of starting another", data_dir.display());
            return;
        }
        core::instance::Claim::First { wakes } => wakes,
    };
    let vault_path = data_dir.join("vault");
    let memory = MemoryEngine::new(vault_path);
    // Before anything can dial out: give the egress spine somewhere to file the
    // record. `Settings::load` sets the local-only policy on the next line, so the
    // window in which the switch isn't enforced is the two statements it takes to
    // read it off disk, during which nothing has run yet.
    core::egress::set_data_dir(&data_dir);
    let settings = Settings::load(&data_dir);

    // Seed a welcome memory on first run so the brain isn't empty.
    if memory.list().is_empty() {
        let _ = memory.save(&core::memory::MemoryInput {
            filename: Some("welcome.md".into()),
            title: "Welcome to Superuser".into(),
            category: "System".into(),
            tags: vec!["welcome".into(), "getting-started".into()],
            summary: "Your private big brain is ready.".into(),
            body: "This is your local Memory Tree. Everything here lives on your machine. \
                   Connect integrations to pull in your Gmail, GitHub, Slack, Calendar and Notion, \
                   then ask questions in Chat — answers are grounded in these memories with citations."
                .into(),
            origin: core::memory::Origin::Internal, // written by the app about itself
            key: None,
            supersedes: None,
        });
    }

    let state = AppState {
        memory,
        settings: Mutex::new(settings),
        registry: Registry::new(),
        data_dir,
        mcp: mcp::McpHost::new(),
        cu_cancel: Arc::new(AtomicBool::new(false)),
        api: Arc::new(Mutex::new(None)),
        dictation: core::dictation::Dictation::default(),
        speaker: core::tts::Speaker::default(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(move |app| setup(app, wakes))
        .invoke_handler(invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while running Superuser")
        .run(|_app, event| {
            // Dev servers started from the browser panel are our children; letting them
            // outlive the app would leave node holding a port with no way to reach it.
            if matches!(event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
                core::devserver::stop_all();
                core::procs::stop_all();
                // The assistants' browsers too. The second event finds nothing left to stop.
                tauri::async_runtime::block_on(core::agentbrowser::computer::stop_all(std::time::Duration::from_secs(3)));
            }
        });
}
