
#[repr(C)]
struct TlsIndex {
    module: usize,
    offset: usize,
}

#[repr(C)]
struct PerCpuBlock {
    tls_block: usize,
}

static BASE_LOAD_ADDR: usize = 0;
#[no_mangle]
unsafe extern "C" fn __tls_get_addr(idx: &'static TlsIndex) -> *const u8 {
    let tls_block: usize;
    asm! {
        "xor %rax, %rax",
        "mov %gs:0(%rax), %rax",
        out("rax") tls_block,
        options(att_syntax),
    }
    (tls_block + idx.offset) as *const u8
}

#[macro_export]
macro_rules! percpu {
    (@percpu [$(pub $(($($vis:tt)*))?)?] $name:ident : $ty:ty = $init:expr) => {
        $(pub $(($($vis)*))*)* static $name: $crate::percpu::PerCpu<$ty> = {
            #[inline(always)]
            fn get() -> *mut $ty {
                #[thread_local]
                static mut VALUE: $ty = $init;
                unsafe { &mut VALUE }
            }
            $crate::percpu::PerCpu {
                get,
            }
        };
    };
    ($(pub $(($($vis:tt)*))?)? static mut ref $name:ident : $ty:ty = $init:expr; $($rest:tt)*) => {
        percpu!(@percpu [$(pub $(($($vis)*))*)*] $name : $ty = $init);
        percpu!($($rest)*);
    };
    () => {};
}

pub struct PerCpu<T: 'static> {
    pub get: fn() -> *mut T,
}

impl<T: 'static> PerCpu<T> {
    #[inline]
    pub fn with<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> R {
        let r = unsafe { &mut *self.get() };
        f(r)
    }

    #[inline]
    pub fn get(&self) -> *mut T {
        (self.get)()
    }
}
