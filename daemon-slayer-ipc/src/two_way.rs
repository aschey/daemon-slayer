use std::io;

use futures::{
    stream::{AbortHandle, Abortable},
    Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt,
};
use tarpc::{transport::channel::UnboundedChannel, ClientMessage, Response};

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) enum TwoWayMessage<Req, Resp> {
    Request(tarpc::ClientMessage<Req>),
    Response(tarpc::Response<Resp>),
}

pub(crate) fn spawn_twoway<Req1, Resp1, Req2, Resp2, T>(
    transport: T,
) -> (
    UnboundedChannel<ClientMessage<Req1>, Response<Resp1>>,
    UnboundedChannel<Response<Resp2>, ClientMessage<Req2>>,
)
where
    T: Stream<Item = Result<TwoWayMessage<Req1, Resp2>, io::Error>>,
    T: Sink<TwoWayMessage<Req2, Resp1>, Error = io::Error>,
    T: Unpin + Send + 'static,
    Req1: Send + 'static,
    Resp1: Send + 'static,
    Req2: Send + 'static,
    Resp2: Send + 'static,
{
    let (server, server_ret) = tarpc::transport::channel::unbounded();
    let (client, client_ret) = tarpc::transport::channel::unbounded();
    let (mut server_sink, server_stream) = server.split();
    let (mut client_sink, client_stream) = client.split();
    let (transport_sink, mut transport_stream) = transport.split();

    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    // Task for inbound message handling
    tokio::spawn(async move {
        while let Some(msg) = transport_stream.next().await {
            match msg.unwrap() {
                TwoWayMessage::Request(req) => server_sink.send(req).await.unwrap(),
                TwoWayMessage::Response(resp) => client_sink.send(resp).await.unwrap(),
            }
        }

        abort_handle.abort();
    });

    let abortable_sink_channel = Abortable::new(
        futures::stream::select(
            server_stream.map_ok(TwoWayMessage::Response),
            client_stream.map_ok(TwoWayMessage::Request),
        )
        .map_err(|e| e.to_string()),
        abort_registration,
    );

    // Task for outbound message handling
    tokio::spawn(
        abortable_sink_channel
            .forward(transport_sink.sink_map_err(|e| e.to_string()))
            .inspect_ok(|_| println!("transport_sink done"))
            .inspect_err(|e| println!("Error in outbound multiplexing: {}", e)),
    );

    (server_ret, client_ret)
}
