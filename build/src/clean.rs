
pub fn clean_main() {
    let mut cmd = crate::cmd::cargo();
    cmd.arg("clean");
    crate::cmd::status(&mut cmd);
}