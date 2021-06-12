
use duct::cmd;

pub fn clean_main() {
    cmd!("cargo", "clean").run().unwrap();
}
