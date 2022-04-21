pub macro barrier($n:expr) {{
    static BARRIER: chos_lib::sync::SpinOnceCell<chos_lib::sync::SpinBarrier> =
        chos_lib::sync::SpinOnceCell::new();
    BARRIER
        .get_or_init(|| chos_lib::sync::SpinBarrier::new($n))
        .wait();
}}

pub macro do_once($body:expr) {{
    use core::sync::atomic::{AtomicBool, Ordering};
    static ONCE: AtomicBool = AtomicBool::new(false);
    if ONCE.compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
        $body;
    }
}}