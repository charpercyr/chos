
pub trait Filesystem {
    fn name(&self) -> &'static str;
}

pub fn register_filesystem(fs: &'static dyn Filesystem) {

}

pub fn unregister_filesystem(fs: &'static dyn Filesystem) {

}
