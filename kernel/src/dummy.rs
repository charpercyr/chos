macro dummy($name:ident, $msg:expr) {
    #[no_mangle]
    extern "C" fn $name() -> ! {
        unimplemented!($msg)
    }
}

dummy!(fmax, "Floating point not supported");
dummy!(fmin, "Floating point not supported");
dummy!(fmaxf, "Floating point not supported");
dummy!(fminf, "Floating point not supported");
