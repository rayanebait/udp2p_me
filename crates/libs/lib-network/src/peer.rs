use log::{debug, error, info, warn};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use thiserror::Error;

use crate::store;

#[derive(Error, Debug)]
pub enum PeerError {
    #[error("No public key.")]
    NoPublicKey,
    #[error("No usable address")]
    NoPreferredAddr,
    #[error("No valid address")]
    NoAddr,
    #[error("No hash")]
    NoHash,
    #[error("Connection timeout")]
    ResponseTimeout,
    #[error("Unknown Peer")]
    UnknownPeer,
    #[error("Unknown error")]
    Unknown,
    #[error("Invalid packet")]
    InvalidPacket,
    #[error("UnkownExtension")]
    UnknownExtension,
    #[error("Peer timed out")]
    PeerTimedOut,
    #[error("No datum")]
    NoDatum,
    #[error("Invalid name format")]
    InvalidUTF8Name,
    #[error("Name changed")]
    NameChanged,
}

#[derive(Default, Debug, Clone)]
pub struct Peer {
    name: Option<String>,
    addresses: Vec<SocketAddr>,
    root: Option<[u8; 32]>,
    public_key: Option<[u8; 64]>,
    extensions: Option<[u8; 4]>,
    timer: Option<Instant>,
}

impl Peer {
    pub fn new() -> Self {
        Self { ..Peer::default() }
    }

    pub fn set_hash(&mut self, root: Option<[u8; 32]>) -> &mut Self {
        self.root = root;
        self
    }

    pub fn set_public_key(&mut self, public_key: Option<[u8; 64]>) -> &mut Self {
        self.public_key = public_key;
        self
    }
    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }
    pub fn set_extensions(&mut self, extensions: Option<[u8; 4]>) -> &mut Self {
        self.extensions = extensions;
        self
    }

    pub fn add_address(&mut self, address: SocketAddr) -> &mut Self {
        self.addresses.push(address);
        self
    }
    pub fn set_timer(&mut self) -> &mut Self {
        self.timer = Some(Instant::now());
        self
    }

    pub fn get_root_hash(&self) -> Option<[u8; 32]> {
        match &self.root {
            Some(h) => Some(*h),
            None => None,
        }
    }

    pub fn get_public_key(&self) -> Option<[u8; 64]> {
        match &self.public_key {
            Some(key) => return Some(*key),
            None => None,
        }
    }

    pub fn get_addresses(&self) -> Option<&Vec<SocketAddr>> {
        if self.addresses.len() == 0 {
            None
        } else {
            Some(&self.addresses)
        }
    }
    pub fn get_name(&self) -> Option<&String> {
        self.name.as_ref()
    }
    pub fn get_extensions(&self) -> Option<[u8; 4]> {
        *&self.extensions
    }
    pub fn has_timed_out(&self, time_out: u64) -> Result<(), PeerError> {
        match self.timer {
            Some(timer) => {
                let elapsed_dur = timer.elapsed();

                if elapsed_dur > Duration::from_millis(time_out) {
                    return Err(PeerError::PeerTimedOut);
                } else {
                    return Ok(());
                }
            }
            None => return Err(PeerError::UnknownPeer),
        }
    }
}

/*When receiving hello/hello reply packet:
    -Add/create peer into active_peers(in process), 2 cases:
        -peer already exists->reset timer (keep alive)
        -peer doesn't exist->create and add peer.
When receiving any other packet:
    -Check if peer exists, 3 cases:
        -peer doesn't exist->ignore (only print info)
        -peer exists:
            -Internal timer expired: ignore
            -Internal timer not expired: keep alive
            (Peer timer is not checked during download
                so that a download can be done without reseting the timer for every packet)
    - */
#[derive(Default)]
pub struct ActivePeers {
    pub addr_map: HashMap<SocketAddr, String>,
    pub peer_map: HashMap<String, Peer>,
}

impl ActivePeers {
    pub fn build_mutex() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(ActivePeers::default()))
    }
    /*Add normal push pop and in set_... verify if peer exists, if not
    only create peer in set_peer_extensions_and_name not in root and pkey */
    pub fn push(&mut self, peer: &Peer) {
        let peer_clone = peer.clone();
        let name = peer.get_name();
        match name {
            Some(name) => {
                for addr in &peer.addresses {
                    self.addr_map.insert(*addr, name.clone());
                }
                self.peer_map.insert(name.clone(), peer_clone);
            }
            None => {
                debug!("Peer has no name");
                return;
            }
        }
    }
    pub fn pop(&mut self, peer: &Peer) {
        for addr in &peer.addresses {
            self.addr_map.remove(addr);
        }
    }
    pub fn get(&self, sock_addr: SocketAddr) -> Option<&Peer> {
        let peer_name = match self.addr_map.get(&sock_addr) {
            Some(name) => name,
            /*Ignore if peer doesn't exist */
            _ => return None,
        };

        self.peer_map.get(peer_name)
    }
    pub fn get_mut(&mut self, sock_addr: SocketAddr) -> Option<&mut Peer> {
        let peer_name = match self.addr_map.get(&sock_addr) {
            Some(name) => name,
            /*Ignore if peer doesn't exist */
            _ => return None,
        };

        self.peer_map.get_mut(peer_name)
    }
    pub fn lock_and_get(
        active_peers: Arc<Mutex<ActivePeers>>,
        sock_addr: SocketAddr,
    ) -> Option<String> {
        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[lock_and_get] Peers mutex is poisoned {e}");
                panic!("[lock_and_get] Peers mutex is poisoned {e}")
            }
        };
        active_peers.addr_map.get(&sock_addr).cloned()
    }
    pub fn lock_and_push(active_peers: Arc<Mutex<ActivePeers>>, peer: Peer) {
        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[lock_and_push] Peers mutex is poisoned {e}");
                panic!("[] Peers mutex is poisoned {e}")
            }
        };
        active_peers.push(&peer);
    }
    pub fn lock_and_pop(active_peers: Arc<Mutex<ActivePeers>>, peer: &Peer) {
        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[lock_and_pop] Peers mutex is poisoned {e}");
                panic!("Peers mutex is poisoned {e}")
            }
        };
        active_peers.pop(peer);
    }

    /*Checks if there is a peer associated to sock_addr. If yes
    reset timer and return, else create the peer and set its extensions and name.
    Cannot fail. */
    pub fn set_peer_extensions_and_name(
        active_peers: Arc<Mutex<ActivePeers>>,
        sock_addr: SocketAddr,
        extensions: Option<[u8; 4]>,
        name: Vec<u8>,
    ) -> Result<(), PeerError> {
        /*Peers are identified by name */
        let name = match String::from_utf8(name) {
            Ok(name) => name,
            Err(_) => return Err(PeerError::InvalidUTF8Name),
        };

        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[set_peer_extensions_and_name] Peers mutex is poisoned {e}");
                panic!("Peers mutex is poisoned {e}")
            }
        };
        match active_peers.addr_map.get(&sock_addr) {
            Some(stored_name) => {
                // println!("KEEP PEER ALIVE {:?}", peer);
                /*Keep alive */
                info!("EXIST");
                if name != *stored_name {
                    return Err(PeerError::NameChanged);
                }
            }
            None => {
                /*Create peer */
                let mut peer = Peer::new();
                peer.add_address(sock_addr)
                    .set_name(name)
                    .set_extensions(extensions)
                    .set_timer();
                // println!("PUSHING PEER {}", String::from_utf8(name).unwrap());
                active_peers.push(&peer);
                return Ok(());
            }
        };

        match active_peers.peer_map.get_mut(&name) {
            Some(peer) => peer.set_timer(),
            None => return Err(PeerError::Unknown),
        };
        return Ok(());
    }

    pub fn set_peer_root(
        active_peers: Arc<Mutex<ActivePeers>>,
        sock_addr: SocketAddr,
        root: Option<[u8; 32]>,
    ) -> Result<(), PeerError> {
        // TODO : figure out why this mutex gets poisoned sometimes
        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[set_peer_root] Peers mutex is poisoned {e}");
                panic!("Peers mutex is poisoned {e}")
            }
        };
        let peer = match active_peers.get_mut(sock_addr) {
            Some(peer) => peer,
            None => return Err(PeerError::UnknownPeer),
        };
        /*keep alive */
        match peer.has_timed_out(30000) {
            /*Hasn't timed out */
            Ok(()) => {
                peer.set_hash(root);
                return Ok(());
            }
            /*Has timedout */
            Err(PeerError::PeerTimedOut) => {
                let peer_clone = peer.clone();
                /*useless line but it is why can't pop with peer */
                // drop(peer);
                active_peers.pop(&peer_clone);
                return Err(PeerError::PeerTimedOut);
            }
            Err(PeerError::UnknownPeer) => Err(PeerError::UnknownPeer),
            _ => panic!("Shouldn't happen"),
        }
    }
    pub fn set_peer_public_key(
        active_peers: Arc<Mutex<ActivePeers>>,
        sock_addr: SocketAddr,
        public_key: Option<[u8; 64]>,
    ) -> Result<(), PeerError> {
        /*DONE */
        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[set_peer_public_key] Peers mutex is poisoned {e}");
                panic!("Peers mutex is poisoned {e}")
            }
        };
        let peer = match active_peers.get_mut(sock_addr) {
            Some(peer) => peer,
            None => return Err(PeerError::UnknownPeer),
        };

        match peer.has_timed_out(30000) {
            /*Hasn't timed out */
            Ok(()) => {
                peer.set_public_key(public_key);
                return Ok(());
            }
            /*Has timedout */
            Err(PeerError::PeerTimedOut) => {
                let peer_clone = peer.clone();
                /*useless line but it is why can't pop with peer */
                // drop(peer);
                active_peers.pop(&peer_clone);
                return Err(PeerError::PeerTimedOut);
            }
            Err(PeerError::UnknownPeer) => Err(PeerError::UnknownPeer),
            _ => panic!("Shouldn't happen"),
        }
    }
    /*Checks the internal timer attached to peer to see if it is elapsed */
    /*The keep alive is done internally in every other methods  */
    pub fn keep_peer_alive(
        active_peers: Arc<Mutex<ActivePeers>>,
        sock_addr: SocketAddr,
    ) -> Result<(), PeerError> {
        /*DONE */
        let mut active_peers = match active_peers.lock() {
            Ok(active_peers) => active_peers,
            Err(e) => {
                error!("[keep_peer_alive] Peers mutex is poisoned {e}");
                panic!("Peers mutex is poisoned {e}")
            }
        };

        let peer = match active_peers.get_mut(sock_addr) {
            Some(peer) => peer,
            None => return Err(PeerError::UnknownPeer),
        };

        match peer.has_timed_out(30000) {
            /*Hasn't timed out */
            Ok(()) => {
                peer.set_timer();
                return Ok(());
            }
            Err(PeerError::PeerTimedOut) => {
                active_peers.addr_map.remove(&sock_addr);
                return Err(PeerError::PeerTimedOut);
            }
            Err(PeerError::UnknownPeer) => return Err(PeerError::Unknown),
            _ => panic!("Shouldn't happen"),
        }
    }
}
