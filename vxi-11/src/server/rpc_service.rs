use onc_rpc::auth::AuthFlavor;
use onc_rpc::{AcceptedReply, AcceptedStatus, MessageType, ReplyBody, RpcMessage};

const NULL_PROCEDURE: u32 = 0;

///
///
trait Service {
    fn handle_message(
        &self,
        msg: RpcMessage<&[u8], &[u8]>,
    ) -> Result<Box<RpcMessage<&[u8], &[u8]>>, ()> {
        if let Some(body) = msg.call_body() {
            let procedure = body.procedure();
            // Handle NULL procedure
            if procedure == NULL_PROCEDURE {
                let mut buf = Vec::new();
                xdr_codec::pack(&(), &mut buf).unwrap();
                return Ok(Box::new(RpcMessage::new(
                    msg.xid(),
                    MessageType::Reply(ReplyBody::Accepted(AcceptedReply::new(
                        AuthFlavor::AuthNone(None),
                        AcceptedStatus::Success(buf.as_slice()),
                    ))),
                )));
            }
            Ok()
        } else {
            Err(())
        }
    }

    fn on_call(
        &self,
        procedure: u32,
        input: &mut dyn xdr_codec::Read,
        response: &mut dyn xdr_codec::Write,
    );
}
