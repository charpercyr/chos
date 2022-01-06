
#[derive(Debug)]
pub struct CS;

impl CS {
    pub fn get() -> u16 {
        unsafe {
            let value;
            asm! {
                "mov %cs, {:x}",
                out(reg) value,
                options(att_syntax, nomem, nostack),
            }
            value
        }
    }
}