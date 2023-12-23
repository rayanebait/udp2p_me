use std::net::SocketAddr;

#[derive(Clone)]
pub enum Action{
    A,
    /*A hashmap is available in ActivePeers: SocketAddr to Peer */
    SendHello(SocketAddr),
    SendHelloReply([u8;4], SocketAddr),
    ProcessHelloReply(SocketAddr),

    SendDatumWithHash([u8;32], SocketAddr),
    ProcessDatum(Vec<u8>, SocketAddr),

    StorePublicKey(Option<[u8;64]>, SocketAddr),
    StoreRoot(Option<[u8;32]>, SocketAddr),

    SendPublicKey(Option<[u8;64]>, SocketAddr),
    SendPublicKeyReply(Option<[u8;64]>, SocketAddr),
    SendRoot(Option<[u8;32]>, SocketAddr),
    SendRootReply(Option<[u8;32]>, SocketAddr),

    SendError(Vec<u8>, SocketAddr),
    ProcessErrorReply(Vec<u8>, SocketAddr),

    NoOp(SocketAddr)
}

pub struct ActionBuilder {
    
}
