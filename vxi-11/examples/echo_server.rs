struct T;
impl RpcService for T {}

#[async_std::main]
async fn main() {

    let listener = TcpListener::bind(("127.0.0.1", 5000)).await.unwrap();
    let mut incoming = listener
        .incoming()
        .log_warnings(|warn| log::warn!("Listening error: {}", warn))
        .handle_errors(Duration::from_millis(100))
        .backpressure(10);

    let t = Arc::new(T);
    while let Some((token, stream)) = incoming.next().await {
        let peer = stream.peer_addr().unwrap();
        println!("Accepted from: {}", peer);

        task::spawn(async move {
            if let Err(err) = t.serve_tcp(stream).await {
                log::warn!("Error processing client: {}", err)
            }
            drop(token);
        });
    }
}