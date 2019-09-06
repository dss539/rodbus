use crate::{Error, Result};
use crate::requests::*;
use crate::requests_info::*;
use crate::session::Session;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

/// All the possible requests that can be sent through the channel
pub(crate) enum Request {
    ReadCoils(RequestWrapper<ReadCoilsRequest>),
}

/// Wrapper for the requests sent through the channel
///
/// It contains the session ID, the actual request and
/// a oneshot channel to receive the reply.
pub(crate) struct RequestWrapper<T: RequestInfo> {
    id: u16,
    argument : T,
    reply_to : oneshot::Sender<Result<T::ResponseType>>,
}

impl<T: RequestInfo> RequestWrapper<T> {
    pub fn new(id: u16, argument : T, reply_to : oneshot::Sender<Result<T::ResponseType>>) -> Self {
        Self { id, argument, reply_to }
    }
}

/// Channel of communication
///
/// To actually send requests to the channel, the user must create
/// a session send the requests through it.
pub struct Channel {
    tx: mpsc::Sender<Request>,
}

impl Channel {
    pub fn new(addr: SocketAddr, runtime: &Runtime) -> Self {
        let (tx, rx) = mpsc::channel(100);
        runtime.spawn(Self::run(rx, addr));
        Channel { tx  }
    }

    pub fn create_session(&self, id: u16) -> Session {
        Session::new(id, self.tx.clone())
    }

    async fn run(rx: mpsc::Receiver<Request>, addr: SocketAddr)  {
        // TODO: if ChannelServer could implement Future itself, we wouldn't need this method.
        // We could simply `runtime.spawn(ChannelServer::new(...))`.
        let mut server = ChannelServer::new(rx, addr);
        server.run().await;
    }
}

const MAX_PDU_SIZE: usize = 253;
const MBAP_SIZE: usize = 7;
const MAX_ADU_SIZE: usize = MAX_PDU_SIZE + MBAP_SIZE;

/// Channel loop
///
/// This loop handles the requests one by one. It serializes the request
/// and sends it through the socket. It then waits for a response, deserialize
/// it and sends it back to the oneshot provided by the caller.
struct ChannelServer {
    addr: SocketAddr,
    rx: mpsc::Receiver<Request>,
    socket: Option<TcpStream>,
    buffer: Vec<u8>,
}

impl ChannelServer {
    pub fn new(rx: mpsc::Receiver<Request>, addr: SocketAddr) -> Self {
        Self { addr, rx, socket: None, buffer: Vec::with_capacity(MAX_ADU_SIZE) }
    }

    pub async fn run(&mut self) {
        while let Some(req) =  self.rx.recv().await {
            match req {
                Request::ReadCoils(req) => self.handle_request(req).await,
            };
        }
    }

    async fn handle_request<Req: RequestInfo>(&mut self, req: RequestWrapper<Req>) {
        let result = self.handle(&req).await;
        req.reply_to.send(result);
    }

    async fn handle<Req: RequestInfo>(&mut self, req: &RequestWrapper<Req>) -> Result<Req::ResponseType> {
        self.try_open_socket().await;
        if let Some(socket) = &mut self.socket {
            self.buffer.clear();
            req.argument.serialize(self.buffer.as_mut_slice());
            socket.write(self.buffer.as_slice()).await.map_err(|_| Error::Tx)?;
            socket.read(self.buffer.as_mut_slice()).await.map_err(|_| Error::Rx)?;
            Req::ResponseType::parse(self.buffer.as_slice()).ok_or(Error::Rx)
        }
        else {
            Err(Error::Connect)
        }
    }

    async fn try_open_socket(&mut self) {
        if self.socket.is_none() {
            self.socket = TcpStream::connect(self.addr).await.ok();
        }
    }
}