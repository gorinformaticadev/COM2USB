use std::fs;
use std::path::PathBuf;

pub const TRAY_ON_PNG: &[u8] = include_bytes!("../icons/tray.png");
pub const TRAY_OFF_PNG: &[u8] = include_bytes!("../icons/tray-off.png");

/// Leitura/escrita do COM2USB.INI no mesmo formato do original
/// ([Settings] com Porta, Velocidade, Paridade, BitsDados, StopsBits, Ativo).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub porta: String,
    pub velocidade: u32,
    pub paridade: String,
    pub bits_dados: u8,
    pub stops_bits: u8,
    pub ativo: bool,
    pub gravar: bool,
    pub deduplicar: bool,
    pub digitar: bool,
    pub enter: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            porta: "COM4".into(),
            velocidade: 9600,
            paridade: "sem".into(),
            bits_dados: 8,
            stops_bits: 2,
            ativo: false,
            gravar: true,
            deduplicar: true,
            digitar: true,
            enter: true,
        }
    }
}

pub fn ini_path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("COM2USB.INI")))
}

struct Ini {
    sections: Vec<(String, Vec<(String, String)>)>,
}

impl Ini {
    fn parse(text: &str) -> Self {
        let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();
        let mut current = String::new();
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with(';') {
                continue;
            }
            if l.starts_with('[') && l.ends_with(']') {
                current = l[1..l.len() - 1].to_string();
                if !sections.iter().any(|(n, _)| n == &current) {
                    sections.push((current.clone(), Vec::new()));
                }
                continue;
            }
            if let Some((k, v)) = l.split_once('=') {
                if !sections.iter().any(|(n, _)| n == &current) {
                    sections.push((current.clone(), Vec::new()));
                }
                if let Some((_, kv)) = sections.iter_mut().find(|(n, _)| n == &current) {
                    kv.retain(|(ek, _)| ek != k.trim());
                    kv.push((k.trim().to_string(), v.trim().to_string()));
                }
            }
        }
        Self { sections }
    }

    fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(section))?
            .1
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    fn string(&self, section: &str, key: &str, default: &str) -> String {
        match self.get(section, key) {
            Some(v) => {
                let t = v.trim_matches(|c| c == '"' || c == '\'').trim();
                if t.is_empty() {
                    default.to_string()
                } else {
                    t.to_string()
                }
            }
            None => default.to_string(),
        }
    }

    fn int(&self, section: &str, key: &str, default: i64) -> i64 {
        self.get(section, key)
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(default)
    }

    fn boolean(&self, section: &str, key: &str, default: bool) -> bool {
        match self.string(section, key, if default { "1" } else { "0" }).to_lowercase().as_str() {
            "0" | "nao" | "não" | "no" | "false" => false,
            _ => true,
        }
    }

    fn set(&mut self, section: &str, key: &str, value: &str) {
        if !self.sections.iter().any(|(n, _)| n.eq_ignore_ascii_case(section)) {
            self.sections.push((section.to_string(), Vec::new()));
        }
        if let Some((_, kv)) = self
            .sections
            .iter_mut()
            .find(|(n, _)| n.eq_ignore_ascii_case(section))
        {
            if let Some(entry) = kv.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
                entry.1 = value.to_string();
            } else {
                kv.push((key.to_string(), value.to_string()));
            }
        }
    }

    fn to_text(&self) -> String {
        let mut out = String::new();
        for (name, kv) in &self.sections {
            out.push_str(&format!("[{}]\n", name));
            for (k, v) in kv {
                out.push_str(&format!("{}={}\n", k, v));
            }
        }
        out
    }

    fn load_file() -> Self {
        let text = ini_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .unwrap_or_default();
        Self::parse(&text)
    }

    fn update(key: &str, value: &str) -> Result<(), String> {
        let p = ini_path().ok_or("não foi possível resolver o COM2USB.INI")?;
        let text = fs::read_to_string(&p).unwrap_or_default();
        let mut ini = Self::parse(&text);
        ini.set("Settings", key, value);
        fs::write(&p, ini.to_text()).map_err(|e| format!("falha ao gravar COM2USB.INI: {}", e))
    }
}

pub fn load_config() -> AppConfig {
    let ini = Ini::load_file();
    AppConfig {
        porta: ini.string("Settings", "Porta", "COM4"),
        velocidade: ini.int("Settings", "Velocidade", 9600).clamp(300, 1_000_000) as u32,
        paridade: ini.string("Settings", "Paridade", "sem"),
        bits_dados: ini.int("Settings", "BitsDados", 8).clamp(5, 8) as u8,
        stops_bits: ini.int("Settings", "StopsBits", 2).clamp(1, 2) as u8,
        ativo: ini.int("Settings", "Ativo", 0) == 1,
        gravar: ini.boolean("Settings", "Gravar", true),
        deduplicar: ini.boolean("Settings", "Deduplicar", true),
        digitar: ini.boolean("Settings", "Digitar", true),
        enter: ini.boolean("Settings", "Enter", true),
    }
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let p = ini_path().ok_or("não foi possível resolver o COM2USB.INI")?;
    let text = fs::read_to_string(&p).unwrap_or_default();
    let mut ini = Ini::parse(&text);
    ini.set("Settings", "Porta", &cfg.porta);
    ini.set("Settings", "Velocidade", &cfg.velocidade.to_string());
    ini.set("Settings", "Paridade", &cfg.paridade);
    ini.set("Settings", "BitsDados", &cfg.bits_dados.to_string());
    ini.set("Settings", "StopsBits", &cfg.stops_bits.to_string());
    ini.set("Settings", "Ativo", if cfg.ativo { "1" } else { "0" });
    ini.set("Settings", "Gravar", if cfg.gravar { "1" } else { "0" });
    ini.set("Settings", "Deduplicar", if cfg.deduplicar { "1" } else { "0" });
    ini.set("Settings", "Digitar", if cfg.digitar { "1" } else { "0" });
    ini.set("Settings", "Enter", if cfg.enter { "1" } else { "0" });
    fs::write(&p, ini.to_text()).map_err(|e| format!("falha ao gravar COM2USB.INI: {}", e))
}

pub fn save_flag(key: &str, value: &str) -> Result<(), String> {
    Ini::update(key, value)
}
