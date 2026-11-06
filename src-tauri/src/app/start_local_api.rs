use super::*;

/// The GUI (surfaces/gui) is an HTTP client, so the local API is the app's own
/// backend — it starts with the app rather than on demand. It stays loopback-only
/// and token-guarded; the token is injected into the window (never written to
/// disk).
pub(super) fn start_local_api(handle: tauri::AppHandle) {
    use tauri::Manager;
    // Supervised, because this is the one task whose death takes the whole app with it: the
    // GUI is an HTTP client of this server. If it stops serving without being asked, it is
    // started again — on the same port when it can be, on another when not — and the window
    // is pointed at wherever it landed. `api_stop` is an ask, and is not restarted.
    tauri::async_runtime::spawn(async move {
        let mut backoff = core::supervise::Backoff::default();
        let mut shown_error = false;
        loop {
            let began = std::time::Instant::now();
            let why = match core::api_server::start(handle.clone(), 8765).await {
                Ok(mut api) => {
                    let (port, token) = (api.port, api.token.clone());
                    log_line(
                        &handle,
                        &if api.fell_back {
                            format!("local API on 127.0.0.1:{port} (8765 was taken by another process)")
                        } else {
                            format!("local API on 127.0.0.1:{port}")
                        },
                    );
                    if let Some(w) = handle.get_webview_window("main") {
                        // The GUI reads these on every request, so a restarted server on a new
                        // port is picked up without a reload. A boot error left from a failed
                        // first attempt goes, because it is no longer true.
                        let js = format!(
                            "window.__SUPERUSER_HTTP__='http://127.0.0.1:{port}';\
                             window.__SUPERUSER_WS__='ws://127.0.0.1:{port}';\
                             window.__SUPERUSER_API_TOKEN__='{token}';\
                             (function(){{var e=document.getElementById('su-boot-error');if(e)e.remove();}})();"
                        );
                        let _ = w.eval(&js);
                    }
                    let ended = api.ended.take();
                    *handle.state::<AppState>().api.lock().unwrap() = Some(api);
                    let Some(ended) = ended else { return };
                    match ended.await {
                        // Asked to stop, or the process is going away with it.
                        Ok(core::api_server::Ending::Stopped) | Err(_) => return,
                        Ok(core::api_server::Ending::Failed(why)) => {
                            let state = handle.state::<AppState>();
                            let mut slot = state.api.lock().unwrap();
                            if slot.as_ref().is_some_and(|a| a.port == port && a.token == token) {
                                *slot = None;
                            }
                            why
                        }
                    }
                }
                // A failure here is total: the GUI is an HTTP client and this is the server it
                // talks to, so without it the window loads and then does nothing at all. It used
                // to be reported with this log line ALONE — which a double-clicked .app throws
                // away — so the symptom reaching the user was a blank, silent app with the cause
                // written somewhere they will never look. Say it on the screen they are staring
                // at. Plain DOM rather than anything in the GUI's own code, because the GUI in
                // this state cannot load its own data and may not have mounted.
                Err(e) => {
                    log_line(&handle, &format!("local API failed to start: {e}"));
                    if !shown_error {
                        shown_error = true;
                        if let Some(w) = handle.get_webview_window("main") {
                            let detail = e.replace('\\', "\\\\").replace('\'', "\\'");
                            let _ = w.eval(&format!(
                                r#"window.__SUPERUSER_BOOT_ERROR__='{detail}';
                                (function(){{
                                  function show(){{
                                    if (document.getElementById('su-boot-error')) return;
                                    var d=document.createElement('div');
                                    d.id='su-boot-error';
                                    d.setAttribute('role','alert');
                                    d.style.cssText='position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;padding:32px;background:#0B0D0F;color:#E8EAED;font:14px/1.6 -apple-system,BlinkMacSystemFont,system-ui,sans-serif';
                                    d.innerHTML='<div style="max-width:520px">'
                                      +'<div style="font-size:16px;font-weight:600;margin-bottom:10px">Superuser could not start its local service</div>'
                                      +'<div style="color:#8B9299;margin-bottom:14px">Everything in this app runs through a server on your own machine. It could not start, so nothing here will load.</div>'
                                      +'<div style="color:#8B9299;margin-bottom:14px">This is almost always another copy of Superuser already running. Quit it and open this one again.</div>'
                                      +'<pre style="white-space:pre-wrap;word-break:break-word;background:#121517;border:1px solid #1A1E21;border-radius:8px;padding:10px 12px;color:#8B9299;font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace;margin:0">'
                                      + window.__SUPERUSER_BOOT_ERROR__ + '</pre></div>';
                                    document.body.appendChild(d);
                                  }}
                                  if (document.body) show();
                                  else document.addEventListener('DOMContentLoaded', show);
                                }})();"#
                            ));
                        }
                    }
                    e
                }
            };
            let wait = backoff.after(began.elapsed());
            core::supervise::record("the local API", &why);
            log_line(&handle, &format!("local API stopped ({why}) — starting it again in {}s", wait.as_secs()));
            tokio::time::sleep(wait).await;
        }
    });
}

pub(super) fn log_line(_handle: &tauri::AppHandle, msg: &str) {
    eprintln!("[superuser] {msg}");
}
