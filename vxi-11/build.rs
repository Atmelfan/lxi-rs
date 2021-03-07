use xdrgen;

fn main() {
    xdrgen::compile("src/protocol/vxi11.x").expect("xdrgen vxi11.x failed");
}