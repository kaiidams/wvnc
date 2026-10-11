#[link(wasm_import_module = "env")]
unsafe extern "C" {
    pub(crate) fn debug_print(level: u32, ptr: *const u8, len: usize) -> u32;
    pub(crate) fn set_title(ptr: *const u8, len: usize) -> bool;
    pub(crate) fn resize(width: u16, height: u16) -> bool;
    pub(crate) fn fill_rect(pixel: u32, x: u16, y: u16, width: u16, height: u16) -> bool;
    pub(crate) fn update_rect(ptr: *const u8, x: u16, y: u16, width: u16, height: u16) -> bool;
    pub(crate) fn copy_rect(sx: u16, sy: u16, dx: u16, dy: u16, width: u16, height: u16) -> bool;
    pub(crate) fn bell() -> bool;
    pub(crate) fn clipboard_text(ptr: *const u8, len: usize) -> bool;
    pub(crate) fn led(state: u8) -> bool;
    pub(crate) fn pointer_mode(mode: u8) -> bool;
    pub(crate) fn audio_start() -> bool;
    pub(crate) fn audio_data(ptr: *const u8, len: usize) -> bool;
    pub(crate) fn audio_stop() -> bool;
    pub(crate) fn socket_open() -> bool;
    pub(crate) fn socket_close() -> bool;
    pub(crate) fn socket_send(ptr: *const u8, len: usize) -> bool;
    pub(crate) fn password(ptr: *mut u8, len: usize) -> bool;
}
