use tower::Service;

struct Rpc {

}

struct RpcRequest {
    program: usize,
    proc: usize,
    vers: usize,
    port: usize,
    data: Vec<u8>
}

struct RpcResponse {
    program: usize,
    proc: usize,
    vers: usize,
    port: usize,
    data: Vec<u8>
}
