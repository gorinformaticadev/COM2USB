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
    port: Arc<Mutex<Option<Box<dyn serialport::SerialPort>>>>,
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

fn open_port(cfg: &AppConfig) -> Result<Box<dyn serialport::SerialPort>, String> {
    let name = windows_path(&cfg.porta);
    serialport::new(&name, cfg.velocidade)
        .data_bits(parse_data_bits(cfg.bits_dados))
        .parity(parse_parity(&cfg.paridade))
        .stop_bits(parse_stop_bits(cfg.stops_bits))
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|e| format!("{} indisponível: {}", name, e))
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
            port: Arc::new(Mutex::new(None)),
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

    pub fn activate(&mut self, app: &tauri::AppHandle) -> Result<(), String> {
        if self.is_active() {
            return Ok(());
        }
        // espera o thread anterior terminar antes de reabrir a porta
        self.await_previous_thread();
        {
            let mut guard = self.port.lock().unwrap();
            if guard.is_none() {
                *guard = Some(open_port(&self.config)?);
            }
        }
        set_status(&self.shared, true, &self.config.porta.clone(), None);
        {
            let mut last = self.shared.last_code.lock().unwrap();
            *last = None;
        }
        self.active.store(true, Ordering::SeqCst);
        self.done.store(false, Ordering::SeqCst);

        let active = self.active.clone();
        let done = self.done.clone();
        let port_arc = self.port.clone();
        let shared = self.shared.clone();
        let app2 = app.clone();
        let port_name = self.config.porta.clone();

        let handle = std::thread::spawn(move || {
            let mut buf: Vec<u8> = Vec::new();
            let mut byte = [0u8; 1];
            let mut last_byte_at = Instant::now();
            while active.load(Ordering::SeqCst) {
                let read_result = {
                    let mut guard = port_arc.lock().unwrap();
                    match guard.as_mut() {
                        Some(p) => p.read(&mut byte),
                        None => break,
                    }
                };
                match read_result {
                    Ok(1) => {
                        last_byte_at = Instant::now();
                        buf.push(byte[0]);
                        let term = byte[0] == b'\n' || byte[0] == b'\r';
                        if term || buf.len() >= MAX_LINE {
                            flush_line(&mut buf, &app2, &shared, &port_name);
                        }
                    }
                    Ok(_) => {
                        flush_line(&mut buf, &app2, &shared, &port_name);
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::TimedOut
                            || e.kind() == std::io::ErrorKind::WouldBlock
                        {
                            if !buf.is_empty() && last_byte_at.elapsed() >= SILENCE_FLUSH {
                                flush_line(&mut buf, &app2, &shared, &port_name);
                            }
                        } else {
                            set_status(&shared, false, "", Some(e.to_string()));
                            let _ = app2.emit("serial-error", e.to_string());
                            break;
                        }
                    }
                }
            }
            flush_line(&mut buf, &app2, &shared, &port_name);
            done.store(true, Ordering::SeqCst);
        });
        self.handle = Some(handle);
        Ok(())
    }

    fn await_previous_thread(&mut self) {
        if let Some(h) = self.handle.take() {
            for _ in 0..100 {
                if self.done.load(Ordering::SeqCst) || h.is_finished() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if self.done.load(Ordering::SeqCst) || h.is_finished() {
                let _ = h.join();
            }
            // se ainda vivo, o handle é descartado e o thread segue destacado
        }
    }

    pub fn deactivate(&mut self) {
        if !self.is_active() {
            self.await_previous_thread();
            return;
        }
        self.active.store(false, Ordering::SeqCst);
        self.await_previous_thread();
    }
}
