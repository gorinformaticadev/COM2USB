use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct KeybdInput {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
union InputData {
    ki: KeybdInput,
    padding: [u8; 32],
}

#[repr(C)]
struct Input {
    input_type: u32,
    data: InputData,
}

#[link(name = "user32")]
extern "system" {
    fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
    fn GetCurrentProcessId() -> u32;
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const KEYEVENTF_UNICODE: u32 = 0x0004;
const VK_RETURN: u16 = 0x0D;

fn teclado(w_vk: u16, scan: u16, flags: u32) -> Input {
    Input {
        input_type: INPUT_KEYBOARD,
        data: InputData {
            ki: KeybdInput {
                w_vk,
                w_scan: scan,
                dw_flags: flags,
                time: 0,
                dw_extra_info: 0,
            },
        },
    }
}

fn send(eventos: &[Input]) {
    if eventos.is_empty() {
        return;
    }
    unsafe {
        SendInput(
            eventos.len() as u32,
            eventos.as_ptr(),
            std::mem::size_of::<Input>() as i32,
        );
    }
}

fn char_eventos(ch: char) -> Vec<Input> {
    let mut out = Vec::new();
    for utf16 in ch.encode_utf16(&mut [0u16; 2]) {
        out.push(teclado(0, *utf16, KEYEVENTF_UNICODE));
        out.push(teclado(0, *utf16, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    out
}

/// Emite um texto como digitação física, exatamente como o SEALST fazia com
/// `keybd_event`. O destino é a janela em foco, então o app de destino
/// (editor, sistema de vendas, planilha) recebe o código sem mudança de foco.
pub fn digitar(texto: &str, com_enter: bool) {
    let mut eventos: Vec<Input> = texto.chars().flat_map(char_eventos).collect();
    if com_enter {
        eventos.push(teclado(VK_RETURN, 0, 0));
        eventos.push(teclado(VK_RETURN, 0, KEYEVENTF_KEYUP));
    }
    send(&eventos);
}

/// A janela em foco pertence a este processo? Digitar no nosso próprio
/// formulário de propriedades não entrega o código a lugar nenhum.
pub fn foco_e_proprio() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        pid == GetCurrentProcessId()
    }
}
