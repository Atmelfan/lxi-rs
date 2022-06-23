use std::process::Command;


#[test]
#[ignore = "External python tests"]
fn pytest() {
    let status = Command::new("pytest")
        //.args(["-s"])
        .status()
        .expect("failed to execute process");
    assert!(status.success(), "Python tests failed");
}
