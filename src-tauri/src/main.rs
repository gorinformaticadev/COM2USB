#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod serial;

#[cfg(windows)]
mod wedge;

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

/// Rastro de inicialização ao lado do executável. Sem ele, um app que morre no
/// logon não deixa sinal algum (subsystem windows, sem console).
fn log_startup(msg: &str) {
    let Ok(exe) = std::env::current_exe() else { return };
    let Some(dir) = exe.parent() else { return };
    let linha = format!(
        "{}\tpid {}\t{}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        std::process::id(),
        msg
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("COM2USB.log"))
    {
        use std::io::Write;
        let _ = f.write_all(linha.as_bytes());
    }
}

fn schtasks(args: &[&str]) -> Result<std::process::Output, String> {
    let mut cmd = std::process::Command::new("schtasks");
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().map_err(|e| format!("falha ao executar schtasks: {}", e))
}

/// O token atual tem privilégios elevados? (define se podemos criar a tarefa sem pedir UAC)
#[cfg(windows)]
fn elevado() -> bool {
    use std::ffi::c_void;
    #[repr(C)]
    struct TokenElevation {
        elevado: u32,
    }
    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            classe: i32,
            info: *mut c_void,
            len: u32,
            retorno: *mut u32,
        ) -> i32;
        fn CloseHandle(objeto: *mut c_void) -> i32;
    }
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION: i32 = 20;
    unsafe {
        let processo = -1isize as *mut c_void; // GetCurrentProcess()
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(processo, TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut dados = TokenElevation { elevado: 0 };
        let mut tamanho = 0u32;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut dados as *mut _ as *mut c_void,
            std::mem::size_of::<TokenElevation>() as u32,
            &mut tamanho,
        );
        CloseHandle(token);
        ok != 0 && dados.elevado == 1
    }
}

/// SID do usuário dono do token atual — a tarefa de logon precisa dele para
/// saber quem vai estar logado quando o Windows terminar de subir.
#[cfg(windows)]
fn sid_do_usuario() -> Option<String> {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SidAndAttributes {
        sid: *mut c_void,
        atributos: u32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct TokenUser {
        usuario: SidAndAttributes,
    }
    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            classe: i32,
            info: *mut c_void,
            len: u32,
            retorno: *mut u32,
        ) -> i32;
        fn ConvertSidToStringSidW(sid: *mut c_void, texto: *mut *mut u16) -> i32;
        fn LocalFree(objeto: *mut c_void) -> *mut c_void;
        fn CloseHandle(objeto: *mut c_void) -> i32;
    }
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_USER: i32 = 1;
    unsafe {
        let processo = -1isize as *mut c_void;
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(processo, TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let necessario = &mut 0u32;
        // primeira chamada só informa o tamanho do buffer
        GetTokenInformation(token, TOKEN_USER, std::ptr::null_mut(), 0, necessario);
        let tamanho = *necessario as usize;
        if tamanho == 0 {
            CloseHandle(token);
            return None;
        }
        let mut buffer = vec![0u8; tamanho];
        let ok = GetTokenInformation(
            token,
            TOKEN_USER,
            buffer.as_mut_ptr() as *mut c_void,
            tamanho as u32,
            necessario,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }
        let usuario = *(buffer.as_ptr() as *const TokenUser);
        let mut texto = std::ptr::null_mut::<u16>();
        if ConvertSidToStringSidW(usuario.usuario.sid, &mut texto) == 0 {
            return None;
        }
        let mut fim = 0usize;
        while *texto.add(fim) != 0 {
            fim += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(texto, fim));
        LocalFree(texto as *mut c_void);
        Some(sid)
    }
}

/// Definição da tarefa de logon. O `schtasks` de linha de comando não expõe
/// quatro pontos que já derrubaram a inicialização automática: bateria, start
/// atrasado quando o agendador estava ocupado, limite padrão de 72 h de
/// execução (que mata um app de bandeja) e retry de falha.
#[cfg(windows)]
fn tarefa_xml(exe: &str, sid: &str) -> String {
    let exe = exe.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>COM2USB - captura do leitor de codigo de barras, oculto na bandeja.</Description>
  </RegistrationInfo>
  <Principals>
    <Principal id="Author">
      <UserId>{sid}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>3</Count>
    </RestartOnFailure>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
      <WaitTimeout>PT1H</WaitTimeout>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <Priority>7</Priority>
  </Settings>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <Delay>PT15S</Delay>
    </LogonTrigger>
  </Triggers>
  <Actions Context="Author">
    <Exec>
      <Command>"{exe}"</Command>
    </Exec>
  </Actions>
</Task>"#
    )
}

/// Grava a definição em UTF-16 LE com BOM, a codificação que o schtasks espera
/// quando a declaração diz encoding="UTF-16".
#[cfg(windows)]
fn gravar_xml_da_tarefa(xml: &str) -> Result<std::path::PathBuf, String> {
    let caminho = std::env::temp_dir().join(format!("com2usb-tarefa-{}.xml", std::process::id()));
    let mut bytes = vec![0xffu8, 0xfe];
    for unidade in xml.encode_utf16() {
        bytes.extend_from_slice(&unidade.to_le_bytes());
    }
    std::fs::write(&caminho, bytes).map_err(|e| format!("falha ao gravar a definição da tarefa: {}", e))?;
    Ok(caminho)
}

/// Roda o schtasks num processo elevado: o Windows pede UAC uma única vez.
#[cfg(windows)]
fn schtasks_elevado(args: &[&str]) -> Result<(), String> {
    use std::process::Stdio;

    let lista = args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");
    let script = format!(
        "$ErrorActionPreference='Stop'\r\n\
         try {{\r\n\
         \x20 $p = Start-Process -FilePath schtasks.exe -ArgumentList @({lista}) -Verb RunAs -WindowStyle Hidden -Wait -PassThru\r\n\
         \x20 exit $p.ExitCode\r\n\
         }} catch {{\r\n\
         \x20 exit 1223\r\n\
         }}\r\n"
    );

    let caminho = std::env::temp_dir().join(format!("com2usb-autostart-{}.ps1", std::process::id()));
    let conteudo = format!("\u{feff}{script}");
    std::fs::write(&caminho, conteudo).map_err(|e| format!("falha ao gravar o script de elevação: {}", e))?;

    let resultado = (|| -> Result<std::process::ExitStatus, String> {
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &caminho.to_string_lossy(),
        ]);
        cmd.stdin(Stdio::null()).stdout(Stdio::null());
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.status().map_err(|e| format!("falha ao solicitar elevação: {}", e))
    })();
    let _ = std::fs::remove_file(&caminho);

    let status = resultado?;
    match status.code() {
        Some(0) => Ok(()),
        Some(1223) => Err("Você cancelou a confirmação do Windows, e a tarefa não foi alterada.".into()),
        Some(5) => Err("O Windows recusou a elevação, e a tarefa não foi alterada.".into()),
        _ => Err("O Windows não permitiu elevar o processo, e a tarefa não foi alterada.".into()),
    }
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
fn list_ports(_app: tauri::AppHandle) -> Vec<String> {
    let mut ports: Vec<String> = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.port_name)
        .collect();
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
fn save_settings(app: tauri::AppHandle, mut settings: AppConfig) -> Result<StatusOut, String> {
    settings.validate()?;
    settings.ativo = status_of(&app).running;
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

fn erro_tarefa(out: &std::process::Output, acao: &str) -> String {
    let msg = String::from_utf8_lossy(&out.stderr);
    let motivo = msg
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("motivo desconhecido");
    format!("Não foi possível {acao} a tarefa de logon.\n\n{motivo}")
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<bool, String> {
    let ja_registrada = get_autostart();
    if enabled == ja_registrada {
        return Ok(ja_registrada);
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        return Err("Autostart elevado é disponível apenas no Windows.".into());
    }

    #[cfg(windows)]
    {
        let mut xml_temp: Option<std::path::PathBuf> = None;
        let args: Vec<String> = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let sid = sid_do_usuario().ok_or("não foi possível identificar o usuário atual")?;
            let caminho = gravar_xml_da_tarefa(&tarefa_xml(&exe.to_string_lossy(), &sid))?;
            xml_temp = Some(caminho.clone());
            vec![
                "/Create".into(),
                "/F".into(),
                "/TN".into(),
                TASK_NAME.into(),
                "/XML".into(),
                caminho.to_string_lossy().into_owned(),
            ]
        } else {
            vec![
                "/Delete".into(),
                "/TN".into(),
                TASK_NAME.into(),
                "/F".into(),
            ]
        };
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let resultado = if elevado() {
            let out = schtasks(&refs)?;
            if out.status.success() {
                Ok(())
            } else {
                Err(erro_tarefa(&out, if enabled { "criar" } else { "remover" }))
            }
        } else {
            schtasks_elevado(&refs).map_err(|e| {
                format!(
                    "{e}\n\nA tarefa precisa de privilégios administrativos porque roda com o nível mais elevado a cada logon."
                )
            })
        };
        if let Some(p) = xml_temp.as_ref() {
            let _ = std::fs::remove_file(p);
        }
        resultado?;

        let agora = get_autostart();
        if agora == enabled {
            Ok(agora)
        } else {
            Err("A confirmação foi dada, mas a tarefa de logon não mudou de estado.".into())
        }
    }
}

/// Digita um código de exemplo no aplicativo em foco para validar a injeção
/// sem depender do leitor físico.
#[tauri::command]
fn wedge_test(app: tauri::AppHandle, texto: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let com_enter = config::load_config().enter;
        app.run_on_main_thread(move || crate::wedge::digitar(&texto, com_enter))
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (app, texto);
        Err("A digitação do código é um recurso do Windows.".into())
    }
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

/// Modos de linha de comando usados pelo instalador e pelo desinstalador, que
/// já estão elevado: criar ou apagar a tarefa de logon com o mesmo código do
/// checkbox "Iniciar com o Windows", sem abrir interface nenhuma.
fn autostart_por_linha_de_comando() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let registrar = args
        .iter()
        .any(|a| a.eq_ignore_ascii_case("--registrar-autostart"));
    let remover = args
        .iter()
        .any(|a| a.eq_ignore_ascii_case("--remover-autostart"));
    if !(registrar || remover) {
        return None;
    }
    // Registrar significa "a tarefa deve apontar para este executável": se já
    // existe uma tarefa (criada por outro caminho, por exemplo), ela é apagada e
    // regravada. O instalador já está elevado, então nada de UAC extra.
    if registrar && get_autostart() {
        let _ = set_autostart(false);
    }
    match set_autostart(registrar) {
        Ok(_) => {
            log_startup(if registrar {
                "autostart registrado pelo instalador"
            } else {
                "autostart removido pelo desinstalador"
            });
            Some(0)
        }
        Err(e) => {
            log_startup(&format!("autostart recusado: {e}"));
            Some(1)
        }
    }
}

fn main() {
    log_startup("iniciando");
    std::panic::set_hook(Box::new(|info| log_startup(&format!("panic: {info}"))));
    if let Some(codigo) = autostart_por_linha_de_comando() {
        std::process::exit(codigo);
    }

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
            save_settings,
            get_autostart,
            set_autostart,
            wedge_test
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let toggle = MenuItem::with_id(&handle, "toggle", "Ativar", true, None::<&str>)?;
            let props =
                MenuItem::with_id(&handle, "props", "Propriedades...", true, None::<&str>)?;
            let quit = MenuItem::with_id(&handle, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(&handle, &[&toggle, &props, &quit])?;
            *handle.state::<ToggleItem>().0.lock().unwrap() = Some(toggle.clone());
            // No logon o explorer pode ainda não ter a bandeja pronta; um único
            // erro ali mataria a inicialização automática silenciosamente.
            let mut bandeja = None;
            for tentativa in 1..=5 {
                let construtor = TrayIconBuilder::with_id("main")
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
                    });
                match construtor.build(app) {
                    Ok(t) => {
                        if tentativa > 1 {
                            log_startup(&format!("bandeja criada na tentativa {tentativa}"));
                        }
                        bandeja = Some(t);
                        break;
                    }
                    Err(e) => {
                        log_startup(&format!("bandeja indisponível (tentativa {tentativa}): {e}"));
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                    }
                }
            }
            if bandeja.is_none() {
                // sem bandeja o usuário não tem menu, mas a captura e a digitação
                // continuam valendo: é preferível a sair do ar calado.
                log_startup("segui sem bandeja visível");
            }
            let _bandeja = bandeja;

            let initial = config::load_config();
            update_tray(&handle, initial.ativo, &status_of(&handle).tooltip);
            #[cfg(windows)]
            log_startup(&format!(
                "pronto; porta={}; elevado={}",
                initial.porta,
                elevado()
            ));
            #[cfg(not(windows))]
            log_startup(&format!("pronto; porta={}", initial.porta));
            if initial.ativo {
                let h = handle.clone();
                std::thread::spawn(move || {
                    // Logo após o logon o driver USB do adaptador ainda pode estar
                    // subindo; uma única tentativa deixava o autostart inativo.
                    for tentativa in 1..=5 {
                        std::thread::sleep(std::time::Duration::from_millis(
                            if tentativa == 1 { 800 } else { 3000 },
                        ));
                        match set_active(h.clone(), true) {
                            Ok(_) => {
                                log_startup(&format!("porta ativa na tentativa {tentativa}"));
                                break;
                            }
                            Err(e) => {
                                log_startup(&format!(
                                    "ativação falhou (tentativa {tentativa}): {e}"
                                ));
                                if tentativa == 5 {
                                    let _ = h.emit("serial-error", e);
                                }
                            }
                        }
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
