pub macro barrier($n:expr) {{
    static BARRIER: chos_lib::sync::SpinOnceCell<chos_lib::sync::SpinBarrier> =
        chos_lib::sync::SpinOnceCell::new();
    BARRIER
        .get_or_init(|| chos_lib::sync::SpinBarrier::new($n))
        .wait();
}}
