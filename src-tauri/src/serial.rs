use crate::config::{self, AppConfig};
use crate::ReadingPayload;
use chrono::Local;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::Emitter;

const MAX_LINE: usize = 4096;
const SILENCE_FLUSH: Duration = Duration::from_millis(150);

pub struct SerialStatus {
    running: bool,
    port: String,
    error: Option<String>,
    tooltip: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatusOut {
    pub running: bool,
    pub port: String,
    pub error: Option<String>,
    pub tooltip: String,
    pub recording: bool,
}

struct Shared {
    status: Mutex<SerialStatus>,
    last_code: Mutex<Option<String>>,
    last_at: AtomicUsize,
}

pub struct ReaderManager {
    config: AppConfig,
    active: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    shared: Arc<Shared>,
}

fn parse_parity(s: &str) -> serialport::Parity {
    match s.trim().to_lowercase().as_str() {
        "par" | "even" | "e" => serialport::Parity::Even,
        "impar" | "odd" | "o" => serialport::Parity::Odd,
        _ => serialport::Parity::None,
    }
}

fn parse_data_bits(v: u8) -> serialport::DataBits {
    match v {
        5 => serialport::DataBits::Five,
        6 => serialport::DataBits::Six,
        7 => serialport::DataBits::Seven,
        _ => serialport::DataBits::Eight,
    }
}

fn parse_stop_bits(v: u8) -> serialport::StopBits {
    match v {
        1 => serialport::StopBits::One,
        _ => serialport::StopBits::Two,
    }
}

/// Nomes COM >=10 precisam do prefixo de dispositivo no Windows.
fn windows_path(porta: &str) -> String {
    let p = porta.trim();
    let num: Option<u32> = p
        .strip_prefix("COM")
        .or_else(|| p.strip_prefix("com"))
        .and_then(|s| s.parse().ok());
    match num {
        Some(n) if n >= 10 => format!("\\\\.\\COM{}", n),
        _ => p.to_string(),
    }
}

/// O `serialport::Error` descarta o código Win32, então a orientação usa a classe do erro.
fn dica_os(e: &serialport::Error) -> &'static str {
    match e.kind {
        serialport::ErrorKind::NoDevice => {
            "\n\nA porta não pôde ser aberta: pode estar em uso por outro programa (outro \
             capturador de código de barras, por exemplo) ou o leitor foi desconectado."
        }
        _ => {
            "\n\nO Windows mostra a porta mas o driver do adaptador está em estado inválido: \
             desconecte e reconecte o cabo USB, ou desative e reative a porta no Gerenciador \
             de Dispositivos."
        }
    }
}

fn open_port(cfg: &AppConfig) -> Result<Box<dyn serialport::SerialPort>, String> {
    let name = windows_path(&cfg.porta);
    serialport::new(&name, cfg.velocidade)
        .data_bits(parse_data_bits(cfg.bits_dados))
        .parity(parse_parity(&cfg.paridade))
        .stop_bits(parse_stop_bits(cfg.stops_bits))
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|e| format!("{} indisponível: {}{}", name, e, dica_os(&e)))
}

/// Abre a porta pelo Win32 **sem tocar na programação de linha** (nada de
/// `SetCommState`). Alguns drivers de USB-serial — é o que se mediu no CH341 na
/// COM7 — recusam `SetCommState` com `ERROR_GEN_FAILURE` por horas, enquanto o
/// caminho de dados continua entregando o código íntegro no perfil que o próprio
/// driver reporta.
#[cfg(windows)]
fn abrir_sem_reprogramar(cfg: &AppConfig) -> Result<std::fs::File, String> {
    use std::ffi::c_void;
    use std::io::Error;
    use std::os::windows::io::FromRawHandle;

    #[repr(C)]
    struct Timeouts {
        intervalo_leitura: u32,
        multiplicador_leitura: u32,
        constante_leitura: u32,
        multiplicador_escrita: u32,
        constante_escrita: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateFileW(
            nome: *const u16,
            acesso: u32,
            compartilhamento: u32,
            seguranca: *mut c_void,
            disposicao: u32,
            flags: u32,
            modelo: *mut c_void,
        ) -> *mut c_void;
        fn SetCommTimeouts(handle: *mut c_void, timeouts: *const Timeouts) -> i32;
    }

    let caminho = windows_path(&cfg.porta);
    let largo: Vec<u16> = caminho.encode_utf16().chain(std::iter::once(0)).collect();
    // devolve o que estiver no buffer e espera no máximo 50 ms pelo primeiro byte
    let timeouts = Timeouts {
        intervalo_leitura: u32::MAX,
        multiplicador_leitura: u32::MAX,
        constante_leitura: 50,
        multiplicador_escrita: 0,
        constante_escrita: 0,
    };
    let handle = unsafe {
        CreateFileW(
            largo.as_ptr(),
            0xC000_0000,
            0,
            std::ptr::null_mut(),
            3,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == -1isize as *mut c_void {
        return Err(format!(
            "{} indisponível: {}",
            caminho,
            Error::last_os_error()
        ));
    }
    if unsafe { SetCommTimeouts(handle, &timeouts) } == 0 {
        let e = Error::last_os_error();
        unsafe { std::fs::File::from_raw_handle(handle as *mut _) };
        return Err(format!("{} indisponível: {}", caminho, e));
    }
    Ok(unsafe { std::fs::File::from_raw_handle(handle as *mut _) })
}

#[cfg(not(windows))]
fn abrir_sem_reprogramar(_cfg: &AppConfig) -> Result<std::fs::File, String> {
    Err("a abertura sem reprogramação de linha é do Windows".into())
}

/// Quem fornece os bytes: o crate com a linha programada, ou o handle cru.
enum Leitor {
    Serial(Box<dyn serialport::SerialPort>),
    Bruto(std::fs::File),
}

impl Leitor {
    fn ler(&mut self, alvo: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Leitor::Serial(p) => p.read(alvo),
            Leitor::Bruto(f) => f.read(alvo),
        }
    }
}

fn set_status(shared: &Arc<Shared>, running: bool, port: &str, error: Option<String>) {
    let tooltip = match &error {
        Some(e) => format!("COM2USB — erro: {}", e),
        None => format!(
            "COM2USB — {} ({})",
            if running { "ativo" } else { "inativo" },
            port
        ),
    };
    let mut g = shared.status.lock().unwrap();
    g.running = running;
    g.port = port.to_string();
    g.error = error;
    g.tooltip = tooltip;
}

/// Digita o código na janela que estiver em foco, como o SEALST fazia via
/// `keybd_event`. O `SendInput` é entregue na fila de mensagens do thread que o
/// chama, por isso a injeção corre no thread principal do app.
#[cfg(windows)]
fn injetar(app: &tauri::AppHandle, codigo: &str, com_enter: bool) {
    let codigo = codigo.to_string();
    let _ = app.run_on_main_thread(move || {
        if crate::wedge::foco_e_proprio() {
            return;
        }
        crate::wedge::digitar(&codigo, com_enter);
    });
}

#[cfg(not(windows))]
fn injetar(_app: &tauri::AppHandle, _codigo: &str, _com_enter: bool) {}

fn rec_file() -> Option<std::fs::File> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("COM2USB.rec"))
        .ok()
}

fn flush_line(
    buf: &mut Vec<u8>,
    app: &tauri::AppHandle,
    shared: &Arc<Shared>,
    port: &str,
) {
    if buf.is_empty() {
        return;
    }
    let bytes = std::mem::take(buf);
    let line = String::from_utf8_lossy(&bytes)
        .trim_matches(|c: char| c == '\r' || c == '\n' || c == '\0')
        .trim()
        .to_string();
    if line.is_empty() {
        return;
    }
    let cfg = config::load_config();
    let now = Local::now();
    let duplicated = {
        let mut last = shared.last_code.lock().unwrap();
        let secs = now.timestamp() as usize;
        let dup = cfg.deduplicar
            && last.as_deref() == Some(line.as_str())
            && secs.saturating_sub(shared.last_at.load(Ordering::SeqCst)) < 2;
        *last = Some(line.clone());
        shared.last_at.store(secs, Ordering::SeqCst);
        dup
    };
    if cfg.gravar {
        if let Some(mut f) = rec_file() {
            let _ = writeln!(
                f,
                "{}\t{}\t{}",
                now.format("%Y-%m-%d %H:%M:%S"),
                line,
                if duplicated { "DUP" } else { "" }
            );
        }
    }
    if cfg.digitar && !duplicated {
        injetar(&app.clone(), &line, cfg.enter);
    }
    set_status(shared, true, port, None);
    let _ = app.emit(
        "leitura",
        ReadingPayload {
            codigo: line,
            data_hora: now.format("%H:%M:%S").to_string(),
            duplicado: duplicated,
        },
    );
}

impl ReaderManager {
    pub fn new(config: AppConfig) -> Self {
        let shared = Arc::new(Shared {
            status: Mutex::new(SerialStatus {
                running: false,
                port: config.porta.clone(),
                error: None,
                tooltip: "COM2USB".into(),
            }),
            last_code: Mutex::new(None),
            last_at: AtomicUsize::new(0),
        });
        Self {
            config,
            active: Arc::new(AtomicBool::new(false)),
            done: Arc::new(AtomicBool::new(true)),
            handle: None,
            shared,
        }
    }

    pub fn status(&self) -> StatusOut {
        let g: MutexGuard<SerialStatus> = self.shared.status.lock().unwrap();
        StatusOut {
            running: g.running,
            port: g.port.clone(),
            error: g.error.clone(),
            tooltip: g.tooltip.clone(),
            recording: config::load_config().gravar,
        }
    }

    pub fn tooltip(&self) -> String {
        self.shared.status.lock().unwrap().tooltip.clone()
    }

    pub fn note_error(&mut self, msg: &str) {
        set_status(&self.shared, false, &self.config.porta.clone(), Some(msg.to_string()));
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn set_config(&mut self, cfg: AppConfig) {
        self.config = cfg;
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Abre a porta e transfere a posse do handle ao thread de leitura. Enquanto
    /// aquele thread existir a porta pertence a ele — é isso que impede dois leitores
    /// de dividirem os bytes de um mesmo escaneamento.
    pub fn activate(&mut self, app: &tauri::AppHandle) -> Result<(), String> {
        if self.is_active() {
            return Ok(());
        }
        if !self.await_previous_thread() {
            return Err(format!(
                "{} ainda está sendo liberada pelo leitor anterior. Aguarde alguns segundos e \
                 tente de novo.",
                self.config.porta
            ));
        }
        let (mut leitor, fallback) = match open_port(&self.config) {
            Ok(port) => (Leitor::Serial(port), false),
            Err(erro) => match abrir_sem_reprogramar(&self.config) {
                Ok(file) => {
                    crate::log_startup(&format!(
                        "programação de linha recusada ({}); abrindo {} sem reprogramar",
                        erro, self.config.porta
                    ));
                    (Leitor::Bruto(file), true)
                }
                Err(_) => return Err(erro),
            },
        };
        set_status(&self.shared, true, &self.config.porta.clone(), None);
        if fallback {
            let porta = self.config.porta.clone();
            self.shared.status.lock().unwrap().tooltip =
                format!("COM2USB — ativo ({}, modo sem reprogramação de linha)", porta);
        }
        {
            let mut last = self.shared.last_code.lock().unwrap();
            *last = None;
        }
        self.active.store(true, Ordering::SeqCst);
        self.done.store(false, Ordering::SeqCst);

        let active = self.active.clone();
        let done = self.done.clone();
        let shared = self.shared.clone();
        let app2 = app.clone();
        let port_name = self.config.porta.clone();

        let handle = std::thread::spawn(move || {
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = [0u8; 256];
            let mut last_byte_at = Instant::now();
            while active.load(Ordering::SeqCst) {
                match leitor.ler(&mut chunk) {
                    Ok(n) if n > 0 => {
                        last_byte_at = Instant::now();
                        for &b in chunk[..n].iter() {
                            buf.push(b);
                            if b == b'\n' || b == b'\r' || buf.len() >= MAX_LINE {
                                flush_line(&mut buf, &app2, &shared, &port_name);
                            }
                        }
                    }
                    Ok(_) => {
                        if !buf.is_empty() && last_byte_at.elapsed() >= SILENCE_FLUSH {
                            flush_line(&mut buf, &app2, &shared, &port_name);
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::TimedOut
                            || e.kind() == std::io::ErrorKind::WouldBlock
                        {
                            if !buf.is_empty() && last_byte_at.elapsed() >= SILENCE_FLUSH {
                                flush_line(&mut buf, &app2, &shared, &port_name);
                            }
                        } else {
                            active.store(false, Ordering::SeqCst);
                            set_status(&shared, false, &port_name, Some(e.to_string()));
                            let _ = app2.emit("serial-error", e.to_string());
                            break;
                        }
                    }
                }
            }
            // um código interrompido pela desativação não é gravado: meio código na lista
            // pareceria leitura errada do leitor.
            drop(leitor);
            done.store(true, Ordering::SeqCst);
        });
        self.handle = Some(handle);
        Ok(())
    }

    /// `true` quando já não existe leitor dono da porta.
    fn await_previous_thread(&mut self) -> bool {
        let Some(h) = self.handle.as_ref() else {
            return true;
        };
        for _ in 0..200 {
            if self.done.load(Ordering::SeqCst) || h.is_finished() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        if self.done.load(Ordering::SeqCst) || h.is_finished() {
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
            true
        } else {
            false
        }
    }

    pub fn deactivate(&mut self) {
        self.active.store(false, Ordering::SeqCst);
        self.await_previous_thread();
        let porta = self.config.porta.clone();
        set_status(&self.shared, false, &porta, None);
    }
}
