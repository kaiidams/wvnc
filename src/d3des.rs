unsafe extern "C" {
    pub(crate) fn deskey(key: *const u8, mode: u8);
    pub(crate) fn des(from: *mut u8, to: *mut u8);
}

pub(crate) const EN0: u32 = 0;
pub(crate) const EN1: u32 = 1;
