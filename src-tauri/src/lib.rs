// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, EventTarget, Manager, RunEvent, Runtime, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

struct AllowExit(AtomicBool);
struct IsBatchFocusing(AtomicBool);
struct NoteRegistry(RwLock<HashSet<String>>);

fn get_session_order<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<String> {
    if let Ok(store) = app.store("session.bin") {
        store
            .get("open_notes")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
            .unwrap_or_else(|| vec![])
    } else {
        vec![]
    }
}

fn update_session_order<R: Runtime>(app: &tauri::AppHandle<R>, note_id: String, remove: bool) {
    if let Ok(store) = app.store("session.bin") {
        let mut order = get_session_order(app);

        order.retain(|id| id != &note_id);
        if !remove {
            order.push(note_id);
        }

        let _ = store.set("open_notes", serde_json::to_value(order).unwrap());
        let _ = store.save();
    }
}

#[tauri::command]
async fn save_note(id: String, content: String, app: tauri::AppHandle) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notes");

    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    fs::write(path.join(format!("{}.md", id)), content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_note(id: String, app: tauri::AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notes")
        .join(format!("{}.md", id));

    if !path.exists() {
        return Ok("".to_string());
    }

    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_note(id: String, app: tauri::AppHandle) -> Result<(), String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notes")
        .join(format!("{}.md", id));

    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }

    update_session_order(&app, id.clone(), true);
    
    // Close window if it's open
    let label = format!("note-{}", id);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.close();
    }

    let _ = app.emit_to(EventTarget::any(), "refresh-notes", ());
    Ok(())
}

#[derive(serde::Serialize)]
struct NoteInfo {
    id: String,
    preview: String,
}

#[tauri::command]
async fn get_all_notes(app: tauri::AppHandle) -> Result<Vec<NoteInfo>, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notes");

    if !path.exists() {
        return Ok(vec![]);
    }

    let mut notes = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let id = path.file_stem().unwrap().to_str().unwrap().to_string();
            let content = fs::read_to_string(&path).unwrap_or_default();
            // Take first 100 chars for preview
            let preview = content.chars().take(100).collect();
            notes.push(NoteInfo { id, preview });
        }
    }
    Ok(notes)
}

#[tauri::command]
async fn open_note_window_cmd(id: String, app: tauri::AppHandle) -> Result<(), String> {
    create_note_window(&app, Some(id), true, true);
    Ok(())
}

#[tauri::command]
async fn create_new_note_cmd(app: tauri::AppHandle) -> Result<(), String> {
    println!("Backend: create_new_note_cmd triggered");
    match create_note_window(&app, None, true, true) {
        Some(_) => {
            println!("Backend: Note window created successfully");
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let _ = handle.emit_to(EventTarget::any(), "refresh-notes", ());
            });
            Ok(())
        },
        None => {
            println!("Backend: Failed to create note window");
            Err("Failed to create note window".to_string())
        }
    }
}

#[tauri::command]
async fn trigger_refresh_notes(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app.emit_to(EventTarget::any(), "refresh-notes", ());
    Ok(())
}

fn create_note_window<R: Runtime>(app: &tauri::AppHandle<R>, id: Option<String>, save: bool, should_show: bool) -> Option<tauri::WebviewWindow<R>> {
    let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let label = format!("note-{}", id);

    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        Some(window)
    } else {
        // Ensure notes directory exists so Dashboard can find it
        if let Ok(path) = app.path().app_data_dir() {
            let notes_path = path.join("notes");
            let _ = fs::create_dir_all(&notes_path);
            
            // If it's a new note, create an empty file so it appears in Dashboard immediately
            let note_file = notes_path.join(format!("{}.md", id));
            if !note_file.exists() {
                let _ = fs::write(note_file, "");
            }
        }

        println!("Building window with label: {}", label);
        let window_res = WebviewWindowBuilder::new(app, label.clone(), tauri::WebviewUrl::App("index.html".into()))
            .title("Sticky Note")
            .inner_size(300.0, 300.0)
            .resizable(true)
            .decorations(false)
            .transparent(true)
            .always_on_top(false)
            .skip_taskbar(true)
            .visible(false)
            .build();

        println!("Window build result for {}: {:?}", label, window_res.as_ref().map(|_| "Ok").map_err(|e| e));

        match window_res {
            Ok(window) => {
                // Register window as a sticky note
                if let Ok(mut registry) = app.state::<NoteRegistry>().0.write() {
                    registry.insert(label.clone());
                }

                let id_for_events = id.clone();
                let label_for_events = label.clone();
                let handle_for_events = app.clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::Focused(true) => {
                        let is_batch = handle_for_events.state::<IsBatchFocusing>();
                        if !is_batch.0.load(Ordering::SeqCst) {
                            update_session_order(&handle_for_events, id_for_events.clone(), false);
                        }
                    }
                    tauri::WindowEvent::Destroyed => {
                        if let Ok(mut registry) = handle_for_events.state::<NoteRegistry>().0.write() {
                            registry.remove(&label_for_events);
                        }
                        update_session_order(&handle_for_events, id_for_events.clone(), true);
                    }
                    _ => {}
                });

                if save {
                    update_session_order(app, id, false);
                }

                if should_show {
                    let _ = window.show();
                }
                
                Some(window)
            },
            Err(e) => {
                println!("Error building window: {:?}", e);
                None
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let new_note_shortcut = Shortcut::new(
        Some(
            tauri_plugin_global_shortcut::Modifiers::ALT
                | tauri_plugin_global_shortcut::Modifiers::SHIFT,
        ),
        tauri_plugin_global_shortcut::Code::KeyN,
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &new_note_shortcut
                        && event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                    {
                        create_note_window(app, None, true, true);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            save_note,
            load_note,
            delete_note,
            get_all_notes,
            open_note_window_cmd,
            create_new_note_cmd,
            trigger_refresh_notes
        ])
        .setup(move |app| {
            app.manage(AllowExit(AtomicBool::new(false)));
            app.manage(IsBatchFocusing(AtomicBool::new(false)));
            app.manage(NoteRegistry(RwLock::new(HashSet::new())));
            app.global_shortcut().register(new_note_shortcut)?;

            // Restore session or create first note (Pro Logic)
            let notes = get_session_order(app.app_handle());
            let handle_for_startup = app.app_handle().clone();
            
            // Perform restoration in an async task to keep the startup process non-blocking
            tauri::async_runtime::spawn(async move {
                if notes.is_empty() {
                    create_note_window(&handle_for_startup, None, true, true);
                } else {
                    let mut restored = Vec::new();
                    for id in notes {
                        if let Some(window) = create_note_window(&handle_for_startup, Some(id), false, false) {
                            restored.push(window);
                        }
                    }
                    // Batch show all restored windows at once
                    for window in restored {
                        let _ = window.show();
                    }
                }
            });

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let new_note_i = MenuItem::with_id(app, "new_note", "New Note", true, None::<&str>)?;
            let dashboard_i = MenuItem::with_id(app, "dashboard", "Open Dashboard", true, None::<&str>)?;
            let open_data_i = MenuItem::with_id(app, "open_data", "Open Data Folder", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &new_note_i,
                    &dashboard_i,
                    &open_data_i,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_i
                ],
            )?;

            // Configure main window (Dashboard) behavior
            if let Some(main_win) = app.get_webview_window("main") {
                let main_win_clone = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        main_win_clone.hide().unwrap();
                        api.prevent_close();
                    }
                });
            }

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let handle = tray.app_handle();
                        
                        // Set batch flag to true to ignore 'Focused' events during this mass operation
                        let is_batch = handle.state::<IsBatchFocusing>();
                        is_batch.0.store(true, Ordering::SeqCst);

                        // 1. Get ONLY windows that are explicitly registered in our NoteRegistry
                        let registry_state = handle.state::<NoteRegistry>();
                        let windows_to_process = {
                            let registry = registry_state.0.read().unwrap();
                            registry.iter()
                                .filter_map(|label| handle.get_webview_window(label))
                                .filter(|w| w.is_visible().unwrap_or(false))
                                .collect::<Vec<_>>()
                        };

                        let mut windows = windows_to_process;


                        let order = get_session_order(handle);

                        let order_map: HashMap<String, usize> = order
                            .iter()
                            .enumerate()
                            .map(|(rank, id)| (id.clone(), rank))
                            .collect();

                        // Sort by session order (bottom to top)
                        windows.sort_by(|a, b| {
                            let id_a = a.label().replace("note-", "");
                            let id_b = b.label().replace("note-", "");
                            let pos_a = order_map.get(&id_a).unwrap_or(&usize::MAX);
                            let pos_b = order_map.get(&id_b).unwrap_or(&usize::MAX);
                            pos_a.cmp(pos_b)
                        });

                        // Capture pin state BEFORE we start manipulation
                        let pin_states: Vec<bool> = windows
                            .iter()
                            .map(|w| w.is_always_on_top().unwrap_or(false))
                            .collect();

                        // Pass 1: "The Hammer" - Bring all to front of OS stack
                        for window in &windows {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_always_on_top(true);
                        }

                        // Pass 2: "The Release" - Restore original pin states
                        // This allows notes to drop back to normal Z-order but stay above other apps
                        for (window, &was_pinned) in windows.iter().zip(pin_states.iter()) {
                            let _ = window.set_always_on_top(was_pinned);
                        }

                        // Final Focus on the topmost (newest) window
                        if let Some(top) = windows.last() {
                            let _ = top.set_focus();
                        }

                        // Use a small delay before resetting batch flag because set_focus is asynchronous
                        // and the 'Focused' event might arrive after this block finishes.
                        let handle_clone = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            let is_batch = handle_clone.state::<IsBatchFocusing>();
                            is_batch.0.store(false, Ordering::SeqCst);
                        });
                    }
                })
                .build(app)?;

            app.manage(menu);

            Ok(())
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                let _ = app.store("session.bin").unwrap().save();
                app.state::<AllowExit>().0.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            "new_note" => {
                create_note_window(app, None, true, true);
            }
            "dashboard" => {
                if let Some(main_win) = app.get_webview_window("main") {
                    let _ = main_win.show();
                    let _ = main_win.unminimize();
                    let _ = main_win.set_focus();
                    let _ = app.emit_to(EventTarget::any(), "refresh-notes", ());
                }
            }
            "open_data" => {
                if let Ok(path) = app.path().app_data_dir() {
                    let _ = tauri_plugin_opener::reveal_item_in_dir(path);
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                let allow_exit = app_handle.state::<AllowExit>();
                if !allow_exit.0.load(Ordering::SeqCst) {
                    // We only prevent the application from exiting. 
                    // Individual windows can still be destroyed (closed).
                    api.prevent_exit();
                }
            }
            _ => {}
        });
}
