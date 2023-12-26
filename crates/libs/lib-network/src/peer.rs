pub mod peer {
    use std::net::SocketAddr;
    use thiserror::Error;
    use std::collections::HashMap;
    use tokio::time::{sleep,Sleep, Duration, Instant};

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
        pub fn set_extensions(&mut self, extensions: Option<[u8;4]>)-> &mut Self {
            self.extensions = extensions;
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
        addr_map: HashMap<SocketAddr, Peer>,
    }

    impl ActivePeers {
        pub fn build_mutex()->Arc<Mutex<Self>>{
            Arc::new(Mutex::new(ActivePeers::default()))
        }
        /*Add normal push pop and in set_... verify if peer exists, if not 
        only create peer in set_peer_extensions_and_name not in root and pkey */
        pub fn push(&mut self, peer:&Peer){
            for addr in &peer.addresses {
                self.addr_map.insert(*addr, peer.clone());
            };
        }
        pub fn pop(&mut self, peer: &Peer){
            for addr in &peer.addresses {
                self.addr_map.remove(addr);
            };
        }
        pub fn lock_and_push(active_peers: Arc<Mutex<ActivePeers>>, peer: Peer){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            active_peers.push(&peer);
        }
        pub fn lock_and_pop(active_peers: Arc<Mutex<ActivePeers>>, peer: &Peer){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            active_peers.pop(peer);
        }

        /*Checks if there is a peer associated to sock_addr. If yes
        reset timer and return, else create the peer and set its extensions and name */
        pub fn set_peer_extensions_and_name(active_peers: Arc<Mutex<ActivePeers>>, sock_addr: SocketAddr,
                                    extensions: Option<[u8;4]>,name: Vec<u8>){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            match active_peers.addr_map.get_mut(&sock_addr){
                Some(peer)=>{
                    peer.set_timer();
                    return;
                },
                _=> {
                    let mut peer = Peer::new();
                    peer.add_address(sock_addr)
                        .set_name(name.clone())
                        .set_extensions(extensions)
                        .set_timer();
                    active_peers.push(&peer);
                    return;
                },
            };
        }

        pub fn set_peer_root(active_peers: Arc<Mutex<ActivePeers>>, sock_addr: SocketAddr,
                                    root: Option<[u8;32]>){
            let mut active_peers = match active_peers.lock(){
                Ok(active_peers)=> active_peers,
                Err(_)=>panic!("Peers mutex is poisoned"),
            };
            let peer = match active_peers.addr_map.get_mut(&sock_addr){
                Some(peer)=> peer,
                /*Ignore if peer doesn't exist */
                _=>return,
            };

            /*keep alive */
            match peer.timer.as_ref().unwrap().is_elapsed() {
                true=>{
                    let peer_clone = peer.clone();
                    /*useless line but it is why can't pop with peer */
                    drop(peer);
                    active_peers.pop(&peer_clone);
                    return;
                },
                false=> {
                    peer.set_timer();
                    peer.root = root;
                }
            }
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

            match peer.timer.as_ref().unwrap().is_elapsed() {
                true=>{
                    /*drop the peer. */
                    let peer_clone = peer.clone();
                    /*useless line but it is why can't pop with peer */
                    drop(peer);
                    active_peers.pop(&peer_clone);
                    return;
                },
                false=> {
                    /*keep alive */
                    peer.set_timer();
                    peer.public_key = public_key;
                }
            }

        }
        /*Checks the internal timer attached to peer to see if it is elapsed */
        /*The keep alive is done internally in every other methods  */
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
