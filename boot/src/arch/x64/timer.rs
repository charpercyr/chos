
use core::time::Duration;
pub fn initialize() {
    
}

pub fn delay(d: Duration) {
    let mut us = d.as_micros();
    while us != 0 {
        us -= 1;
    }
}
