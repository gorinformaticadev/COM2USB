mod config;
mod serial;

use config::{AppConfig, TRAY_OFF_PNG, TRAY_ON_PNG};
use serial::{ReaderManager, StatusOut};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    image::Image,
    tray::TrayIconBuilder,
    Emitter, Manager, Wry,
};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const TASK_NAME: &str = "COM2USB";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn schtasks(args: &[&str]) -> Result<std::process::Output, String> {
    let mut cmd = std::process::Command::new("schtasks");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().map_err(|e| format!("falha ao executar schtasks: {}", e))
}

struct SerialManager(Mutex<ReaderManager>);
struct ToggleItem(Mutex<Option<MenuItem<Wry>>>);

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ReadingPayload {
    codigo: String,
    data_hora: String,
    duplicado: bool,
}

fn tray_label(active: bool) -> &'static str {
    if active {
        "COM2USB — gravando"
    } else {
        "COM2USB — inativo"
    }
}

fn update_tray(app: &tauri::AppHandle, active: bool, tooltip: &str) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(Some(tray_label(active)));
        let _ = tray.set_tooltip(Some(tooltip));
        let bytes = if active { TRAY_ON_PNG } else { TRAY_OFF_PNG };
        if let Ok(img) = Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(img));
        }
    }
}

fn set_toggle_text(app: &tauri::AppHandle, active: bool) {
    let item = app.state::<ToggleItem>().0.lock().unwrap().clone();
    if let Some(mi) = item {
        let _ = mi.set_text(if active { "Desativar" } else { "Ativar" });
    }
}

fn status_of(app: &tauri::AppHandle) -> StatusOut {
    app.state::<SerialManager>().0.lock().unwrap().status()
}

fn show_properties(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("properties") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> AppConfig {
    app.state::<SerialManager>().0.lock().unwrap().config().clone()
}

#[tauri::command]
fn get_status(app: tauri::AppHandle) -> StatusOut {
    status_of(&app)
}

#[tauri::command]
fn list_ports(app: tauri::AppHandle) -> Vec<String> {
    let current = app
        .state::<SerialManager>()
        .0
        .lock()
        .unwrap()
        .config()
        .porta
        .clone();
    let mut ports: Vec<String> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.port_name)
        .collect();
    if !ports.iter().any(|p| *p == current) {
        ports.push(current);
    }
    ports.sort();
    ports
}

#[tauri::command]
fn set_active(app: tauri::AppHandle, ativar: bool) -> Result<StatusOut, String> {
    let tooltip = {
        let state = app.state::<SerialManager>();
        let mut mgr = state.0.lock().map_err(|e| e.to_string())?;
        let result = if ativar {
            mgr.activate(&app)
        } else {
            mgr.deactivate();
            Ok(())
        };
        if let Err(e) = result {
            mgr.note_error(&e);
            let tip = mgr.tooltip();
            drop(mgr);
            update_tray(&app, false, &tip);
            return Err(e);
        }
        mgr.tooltip()
    };
    let _ = config::save_flag("Ativo", if ativar { "1" } else { "0" });
    let status = status_of(&app);
    set_toggle_text(&app, status.running);
    update_tray(&app, status.running, &tooltip);
    Ok(status)
}

#[tauri::command]
fn set_recording(gravar: bool) -> Result<(), String> {
    config::save_flag("Gravar", if gravar { "1" } else { "0" })
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: AppConfig) -> Result<StatusOut, String> {
    settings.validate()?;
    config::save_config(&settings)?;
    let was_active = status_of(&app).running;
    {
        let state = app.state::<SerialManager>();
        let mut mgr = state.0.lock().map_err(|e| e.to_string())?;
        mgr.deactivate();
        mgr.set_config(settings.clone());
        if was_active {
            mgr.activate(&app)?;
        }
    }
    let status = status_of(&app);
    set_toggle_text(&app, status.running);
    update_tray(&app, status.running, &status.tooltip);
    Ok(status)
}

#[tauri::command]
fn get_autostart() -> bool {
    schtasks(&["/Query", "/TN", TASK_NAME])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<bool, String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let tr = format!("\"{}\"", exe.display());
        let out = schtasks(&[
            "/Create", "/F", "/TN", TASK_NAME, "/SC", "ONLOGON", "/RL", "HIGHEST", "/TR", &tr,
        ])?;
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "não foi possível criar a tarefa de logon ({}). Execute o COM2USB como administrador uma vez para habilitar o autostart.",
                msg.trim()
            ));
        }
    } else {
        let _ = schtasks(&["/Delete", "/TN", TASK_NAME, "/F"]);
    }
    Ok(get_autostart())
}

#[tauri::command]
fn open_properties(app: tauri::AppHandle) -> Result<(), String> {
    show_properties(&app);
    Ok(())
}

trait Validate {
    fn validate(&self) -> Result<(), String>;
}

impl Validate for AppConfig {
    fn validate(&self) -> Result<(), String> {
        if self.porta.trim().is_empty() {
            return Err("Porta não informada.".into());
        }
        Ok(())
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_properties(app);
        }))
        .manage(SerialManager(Mutex::new(ReaderManager::new(
            config::load_config(),
        ))))
        .manage(ToggleItem(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_status,
            list_ports,
            set_active,
            set_recording,
            save_settings,
            get_autostart,
            set_autostart,
            open_properties
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let toggle = MenuItem::with_id(&handle, "toggle", "Ativar", true, None::<&str>)?;
            let props =
                MenuItem::with_id(&handle, "props", "Propriedades...", true, None::<&str>)?;
            let quit = MenuItem::with_id(&handle, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(&handle, &[&toggle, &props, &quit])?;
            *handle.state::<ToggleItem>().0.lock().unwrap() = Some(toggle.clone());
            let _tray = TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(TRAY_OFF_PNG)?)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("COM2USB")
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "toggle" => {
                        let active = !status_of(app).running;
                        if let Err(e) = set_active(app.clone(), active) {
                            show_properties(app);
                            let _ = app.emit("serial-error", e);
                        }
                    }
                    "props" => show_properties(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            let initial = config::load_config();
            update_tray(&handle, initial.ativo, &status_of(&handle).tooltip);
            if initial.ativo {
                let h = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    if let Err(e) = set_active(h.clone(), true) {
                        let _ = h.emit("serial-error", e);
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. })
                && window.label() == "properties"
            {
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("falha ao construir o app Tauri")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                // sem código explícito = todas as janelas fechadas; manter vivo na bandeja
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
