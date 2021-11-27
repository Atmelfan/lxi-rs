use std::io::Cursor;

use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_std::task::spawn;
use futures::{
    future,
    io::{ReadHalf, WriteHalf},
    AsyncReadExt, StreamExt,
};

use xdr_codec::{Pack, Unpack};

use crate::rpc::{self, onc_rpc, Error};

use crate::rpc::onc_rpc::write_tcp_message;
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;

#[async_trait]
trait RpcServiceHandler {
    async fn handle_call(&mut self, proc: u32, args: Vec<u8>, res: Vec<u8>) -> rpc::Result<()>;
}

#[derive(Copy, Clone)]
struct RpcService {
    program: u32,
    vers: u32,
    port: u16,
}

struct RpcTcpService<HANDLER> {
    service: RpcService,
    handler: HANDLER,
}

impl<HANDLER> RpcTcpService<HANDLER>
    where
HANDLER: Send + Sync {
    async fn accept(self: Arc<Self>, addr: impl ToSocketAddrs) -> rpc::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        while let Some(stream) = listener
            .incoming()
            .filter_map(|r| future::ready(r.ok())).next().await {
            println!("Accepting from: {}", stream.peer_addr().unwrap());
            let service = self.service.clone();
            let _handle =
                async_std::task::spawn(async move {
                    Self::connection_loop(service, stream)
                });
        }

        Ok(())
    }

    async fn connection_loop(service: RpcService, stream: TcpStream) -> rpc::Result<()> {
        let (mut readhalf, mut writehalf) = stream.split();
        let reader = BufReader::new(&mut readhalf); // 2
        let mut lines = onc_rpc::RpcTcpDeframer::new(reader);

        while let Some(msg) = lines.next().await {
            let mut cur = Cursor::new(msg?);
            let (msg, _s) = onc_rpc::xdr::rpc_msg::unpack(&mut cur).unwrap();
            log::debug!("<- {:?}", msg);
            let xid = msg.xid;
            match msg.body {
                onc_rpc::xdr::_body::REPLY(..) => {
                    panic!()
                }
                onc_rpc::xdr::_body::CALL(body) => {
                    // Unsupported RPC version
                    if body.rpcvers != 2 {
                        write_tcp_message(
                            &mut writehalf,
                            &onc_rpc::xdr::rpc_msg::reply_msg_denied(
                                xid,
                                onc_rpc::xdr::rejected_reply::RPC_MISMATCH(
                                    onc_rpc::xdr::_missmatch_info { high: 2, low: 2 },
                                ),
                            ),
                            (),
                        )
                        .await
                        .unwrap();
                        continue;
                    }
                    // Unsupported authentication
                    // i.e. any authentication because I'm lazy.
                    if body.cred.flavor != onc_rpc::xdr::auth_flavor::AUTH_NONE
                        || body.verf.flavor != onc_rpc::xdr::auth_flavor::AUTH_NONE
                    {
                        write_tcp_message(
                            &mut writehalf,
                            &onc_rpc::xdr::rpc_msg::reply_msg_denied(
                                xid,
                                onc_rpc::xdr::rejected_reply::AUTH_ERROR(
                                    onc_rpc::xdr::auth_stat::AUTH_FAILED,
                                ),
                            ),
                            (),
                        )
                        .await
                        .unwrap();
                        continue;
                    }
                    // Wrong program
                    if body.prog != service.program {
                        write_tcp_message(
                            &mut writehalf,
                            &onc_rpc::xdr::rpc_msg::reply_msg_accepted(
                                xid,
                                onc_rpc::xdr::accepted_reply {
                                    verf: onc_rpc::xdr::opaque_auth::default(),
                                    reply_data: onc_rpc::xdr::_reply_data::PROG_UNAVAIL,
                                },
                            ),
                            (),
                        )
                            .await
                            .unwrap();
                        continue;
                    }
                    // Only supports a single version of program
                    // Because I don't care about anything other than VXI-11 which only have a single version.
                    // It's also 5AM when I'm coding this so fuck it.
                    if body.vers != service.vers {
                        write_tcp_message(
                            &mut writehalf,
                            &onc_rpc::xdr::rpc_msg::reply_msg_accepted(
                                xid,
                                onc_rpc::xdr::accepted_reply {
                                    verf: onc_rpc::xdr::opaque_auth::default(),
                                    reply_data: onc_rpc::xdr::_reply_data::PROG_MISMATCH(
                                        onc_rpc::xdr::_missmatch_info {
                                            high: service.program,
                                            low: service.program,
                                        },
                                    ),
                                },
                            ),
                            (),
                        )
                        .await
                        .unwrap();
                        continue;
                    }



                    unimplemented!()
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service() {

    }
}
