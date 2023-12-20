use crate::congestion_handler::*;
use crate::import_export::handshake_addr;

use prelude::*;

use futures::stream::FuturesUnordered;
use futures::{StreamExt, select};

use tokio::net::UdpSocket;
use std::net::SocketAddr;
use tokio::test;

#[derive(Debug,Clone, Copy)]
pub enum PeerError {
    NoPublicKeyError,
    NoGoodAddrError,
    NoAddrsError,
    NoHashError,
    ResponseTimeoutError,
    UnknownError,
}

impl std::fmt::Display for PeerError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeerError::NoAddrsError => write!(f, "Peer has no address"),
            PeerError::NoPublicKeyError => write!(f, "Peer has no public key"),
            PeerError::NoGoodAddrError => write!(f, "Peer has no good addr"),
            PeerError::NoHashError => write!(f, "Peer has no hash"),
            PeerError::ResponseTimeoutError => write!(f, "Peer didn't respond"),
            PeerError::UnknownError => write!(f, "Unkown peer error"),
        }
    }
}

pub struct ActiveAddrMap{
    map: Rc<HashMap<SocketAddr, PeerData>>,
}

#[derive(Default, Clone)]
pub struct PeerData{
    socketaddrs: Option<Vec<SocketAddr>>,
    good_socketaddrs: Option<Vec<SocketAddr>>,
    hash: Option<[u8;32]>,
    public_key: Option<[u8;64]>,
}

impl PeerData {
    pub fn new()->Self{
        Self{
            ..PeerData::default()
        }
    }

    pub fn set_hash(&mut self, hash: &[u8;32])-> &mut Self{
        self.hash = Some(hash.clone());
        self
    }

    pub fn set_public_key(&mut self, public_key: &[u8;64])-> &mut Self{
        self.public_key = Some(public_key.clone());
        self
    }

    pub fn set_socketaddr(&mut self, socketaddrs: Vec<SocketAddr>)-> &mut Self{
        self.socketaddrs = Some(socketaddrs);
        self
    }

    pub fn get_hash(&self)-> &[u8;32]{
        self.hash.as_ref().unwrap()
    }

    pub fn get_public_key(&self)-> Result<&[u8;64], PeerError>{
        match &self.public_key {
            Some(key)=> return Ok(key),
            None => Err(PeerError::NoPublicKeyError),
        }
    }

    /*Should maybe return a vec addr directly */
    pub fn get_socketaddr(&self)-> Result<&Vec<SocketAddr>, PeerError>{
        match &self.socketaddrs {
            Some(addrs)=> return Ok(addrs),
            None => Err(PeerError::NoAddrsError),
        }
    }

    pub fn get_good_socketaddr(&self)-> Result<&Vec<SocketAddr>, PeerError>{
        match &self.good_socketaddrs {
            Some(addrs)=> return Ok(addrs),
            None => Err(PeerError::NoGoodAddrError),
        }
    }

    pub async fn search_good_addr(&self, sock: UdpSocket,
                 pending: &PendingResponseIds)->Result<(), PeerError>{

        let addresses = self.socketaddrs.as_ref().unwrap();

        let mut wait_response= FuturesUnordered::new();
        for addr in addresses {
            wait_response.push(
                handshake_addr(&sock, addr, &pending);
            );

        }

        /*Wait for the first response
        and return the responses socket addr*/
        let a = wait_response.next().await.unwrap();

        a
    }

    pub fn is_peer_addr(&self, addr: SocketAddr)->bool{
        let peer_has_addr= self.socketaddrs.as_ref()
                .unwrap()
                .contains(&addr);

        peer_has_addr
    }
}
