pub mod peer {
    use std::net::SocketAddr;
    use thiserror::Error;

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
        name: String,
        addresses: Vec<SocketAddr>,
        root_hash: Option<[u8; 32]>,
        public_key: Option<[u8; 64]>,
    }

    impl Peer {
        pub fn new() -> Self {
            Self { ..Peer::default() }
        }

        pub fn set_hash(&mut self, hash: &[u8; 32]) -> &mut Self {
            self.root_hash = Some(hash.clone());
            self
        }

        pub fn set_public_key(&mut self, public_key: &[u8; 64]) -> &mut Self {
            self.public_key = Some(public_key.clone());
            self
        }

        pub fn add_address(&mut self, address: SocketAddr) -> &mut Self {
            self.addresses.push(address);
            self
        }

        pub fn get_root_hash(&self) -> Result<&[u8; 32], PeerError> {
            match &self.root_hash {
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
    }
}
