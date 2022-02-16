
pub trait Filesystem {
    fn name(&self) -> &'static str;
}

pub fn register_filesystem(_fs: &'static dyn Filesystem) {

}

pub fn unregister_filesystem(_fs: &'static dyn Filesystem) {

}
