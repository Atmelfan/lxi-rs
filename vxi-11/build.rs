use xdrgen;

fn main() {
    xdrgen::compile("src/rpc/onc_rpc.x").expect("xdrgen onc_rpc.x failed");
    xdrgen::compile("src/rpc/vxi11.x").expect("xdrgen vxi11.x failed");
    xdrgen::compile("src/rpc/portmap.x").expect("xdrgen portmap.x failed");
}