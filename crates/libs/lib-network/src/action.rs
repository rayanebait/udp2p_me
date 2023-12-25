use std::net::SocketAddr;

#[derive(Clone, Debug)]
pub enum Action{
    A,
    /*A hashmap is available in ActivePeers: SocketAddr to Peer */
    SendNoOp(SocketAddr),
    SendRoot(Option<[u8;32]>, SocketAddr),
    SendError(Vec<u8>, SocketAddr),
    SendHello(Option<[u8;4]>, Vec<u8>,SocketAddr),
    SendPublicKey(Option<[u8;64]>, SocketAddr),
    SendGetDatumWithHash([u8;32], SocketAddr),

    SendHelloReply([u8;4],[u8;4], Vec<u8>, SocketAddr),
    SendRootReply([u8;4], Option<[u8;32]>, SocketAddr),
    SendPublicKeyReply([u8;4],Option<[u8;64]>, SocketAddr),
    SendErrorReply([u8;4],Option<Vec<u8>>, SocketAddr),
    SendDatumWithHash([u8;4], [u8;32], Vec<u8>, SocketAddr),
    
    
    ProcessNoOp(SocketAddr),
    ProcessHello([u8;4],Option<[u8;4]>, Vec<u8>,SocketAddr),
    ProcessError([u8;4],Vec<u8>, SocketAddr),
    ProcessPublicKey([u8;4],Option<[u8;64]>, SocketAddr),
    ProcessRoot([u8;4],Option<[u8;32]>, SocketAddr),
    ProcessGetDatum([u8;4],[u8;32], SocketAddr),
    
    
    ProcessHelloReply(Option<[u8;4]>,Vec<u8>,SocketAddr),
    ProcessErrorReply(Vec<u8>, SocketAddr),
    ProcessRootReply(Option<[u8;32]>, SocketAddr),
    ProcessPublicKeyReply(Option<[u8;64]>, SocketAddr),
    ProcessDatum(Vec<u8>, SocketAddr),

}

