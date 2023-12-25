pub mod peer {
    use std::net::SocketAddr;
    use thiserror::Error;
    use std::collections::HashMap;
    use tokio::time::{sleep,Sleep, Duration};

    use std::sync::{Arc, Mutex};

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
        #[error("Unknown error")]
        Unknown,
    }

    #[derive(Default, Clone)]
    pub struct Peer {
        name: Option<Vec<u8>>,
        addresses: Vec<SocketAddr>,
        root: Option<[u8; 32]>,
        public_key: Option<[u8; 64]>,
        extensions: Option<[u8;4]>,
        timer: Option<Arc<Sleep>>
    }

    impl Peer {
        pub fn new() -> Self {
            Self { ..Peer::default() }
        }

        pub fn set_hash(&mut self, root: &[u8; 32]) -> &mut Self {
            self.root = Some(root.clone());
            self
        }

        pub fn set_public_key(&mut self, public_key: &[u8; 64]) -> &mut Self {
            self.public_key = Some(public_key.clone());
            self
        }
        pub fn set_name(&mut self, name: Vec<u8>)-> &mut Self {
            self.name = Some(name);
            self
        }

        pub fn add_address(&mut self, address: SocketAddr) -> &mut Self {
            self.addresses.push(address);
            self
        }
        pub fn set_timer(&mut self)-> &mut Self {
            self.timer = Some(Arc::new(sleep(Duration::from_secs(160))));
            self
        }

        pub fn get_root_hash(&self) -> Result<&[u8; 32], PeerError> {
            match &self.root {
                Some(h) => Ok(h),
                None => Err(PeerError::NoHash),
            }
        }

        pub fn get_public_key(&self) -> Result<&[u8; 64], PeerError> {
            match &self.public_key {
                Some(key) => return Ok(key),
                None => Err(PeerError::NoPublicKey),
            }
        }

        pub fn get_addresses(&self) -> Result<&Vec<SocketAddr>, PeerError> {
            if self.addresses.len() == 0 {
                return Err(PeerError::NoAddr);
            } else {
                return Ok(&self.addresses);
            }
        }
        pub fn get_name(&self) -> Option<Vec<u8>>{
            self.name.clone()
        }
        pub fn get_extensions(&self) -> Option<[u8;4]>{
            self.extensions.clone()
        }

    }


    #[derive(Default)]
    pub struct ActivePeers {
        addr_map: HashMap<SocketAddr, Peer>,
    }

    impl ActivePeers {
        pub fn build_mutex()->Arc<Mutex<Self>>{
            Arc::new(Mutex::new(ActivePeers::default()))
        }
        /*Add normal push pop and in set_... verify if peer exists, if not 
        only create peer in set_peer_extensions_and_name not in root and pkey */
        pub fn push(active_peers: Arc<Mutex<ActivePeers>>, peer: Peer){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            for addr in &peer.addresses {
                active_peers.addr_map.insert(*addr, peer.clone());
            };
        }
        pub fn pop(active_peers: Arc<Mutex<ActivePeers>>, peer: Peer){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            for addr in &peer.addresses {
                active_peers.addr_map.remove(addr);
            };
        }

        pub fn set_peer_extensions_and_name(active_peers: Arc<Mutex<ActivePeers>>, sock_addr: SocketAddr,
                                    extensions: Option<[u8;4]>,name: Vec<u8>){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            let peer = match active_peers.addr_map.get_mut(&sock_addr){
                Some(peer)=> peer,
                _=>return,
            };

            peer.extensions = extensions;
            peer.name = Some(name); 
        }

        pub fn set_peer_root(active_peers: Arc<Mutex<ActivePeers>>, sock_addr: SocketAddr,
                                    root: Option<[u8;32]>){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            let peer = match active_peers.addr_map.get_mut(&sock_addr){
                Some(peer)=> peer,
                _=>return,
            };

            peer.root = root;
        }
        pub fn set_peer_public_key(active_peers: Arc<Mutex<ActivePeers>>, sock_addr: SocketAddr,
                                    public_key: Option<[u8;64]>){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            let peer = match active_peers.addr_map.get_mut(&sock_addr){
                Some(peer)=> peer,
                _=>return,
            };

            peer.public_key = public_key;
        }
        /*Checks the internal timer attached to peer to see if it is elapsed */
        pub fn keep_peer_alive(active_peers: Arc<Mutex<ActivePeers>>, sock_addr: SocketAddr){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            let mut peer = match active_peers.addr_map.get_mut(&sock_addr){
                Some(peer)=> peer,
                _=>return,
            };

            match &peer.timer {
                Some(timer)=>{
                    if timer.is_elapsed() {
                        active_peers.addr_map.remove(&sock_addr);
                        return;
                    }
                }
                None => return,
            }

            peer.set_timer();
        }

    }

}
