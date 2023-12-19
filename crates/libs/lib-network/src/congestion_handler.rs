use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};

use std::sync::{Mutex, Arc};

use lib_web::discovery::Peer;
use crate::peer_data::*;
use crate::packet::*;

#[derive(Default)]
pub struct ActiveSockets{
    sockets: Vec<SocketAddr>,
}

impl ActiveSockets {
    fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(Self{ sockets: vec![] }))
    }

    fn add_addr(&mut self, sock_addr: SocketAddr){

    }

    fn pop_addr(&mut self, sock_addr: SocketAddr){
        let sock_addr_ind = &mut self.sockets
                                .iter().position(
                                    |addr| addr.eq(&sock_addr)
                                );
    }
}

pub struct ActivePeers {
    peers: Vec<PeerData>,
    addr_map: HashMap<SocketAddr, PeerData>,
}
impl ActivePeers {

    fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(Self{ peers: vec![], addr_map: HashMap::new() }))
    }

    fn add_peer(&mut self, peer: PeerData){

    }

    fn pop_peer(&mut self, peer: PeerData){
        // let sock_addr_ind = &mut self.peers
        //                         .iter().position(
        //                             |peer_data| peer_data.eq(&peer)
        //                         );
    }

}
#[derive(Default)]
pub struct PendingResponseIds{
    pending_packet_ids: Vec<[u8;4]>,
    id_to_peer_map: HashMap<[u8;4], SocketAddr>,
}



#[derive(Debug)]
enum CongestionHandlerError {
    NoPeerWithAddrError,
    NoPacketWithIdError,
}

impl std::fmt::Display for CongestionHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CongestionHandlerError::NoPacketWithIdError=>
                         write!(f, "No pending response for given Id"),
            CongestionHandlerError::NoPeerWithAddrError=>
                         write!(f, "No peer with given socket address"),
        }        
    }
}


impl PendingResponseIds{
    pub fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(PendingResponseIds::default()))
    }
    /*Each time a packet is sent, no access to raw packet so need Packet struct */
    pub fn add_packet_id_raw(&mut self, id: [u8;4], peer_addr: &SocketAddr){

        let mut ids = &mut self.pending_packet_ids;
        ids.push(id.clone());

        let mut map = &mut self.id_to_peer_map;
        map.insert(id.clone(), peer_addr.clone());
    }


    /*Each time a packet is received, it is received as raw bytes so access to Id directly*/
    pub fn pop_packet_id(&mut self, packet_id: &[u8; 4]){
        let mut pending_packet_ids =&mut self.pending_packet_ids;
        let packet_id_ind = pending_packet_ids.iter()
                                .position(
                                    |id| id==packet_id
                                );
                            // .binary_seaArch_by(
                            //     |x| (x).cmp(packet_id)
                            // );
        match packet_id_ind {
            Some(packet_id_ind) => pending_packet_ids.remove(packet_id_ind),
            /*Simply return if Id doesn't match any */
            None => return,
        };
    }   

    pub fn search_id_raw(&self, packet_id: &[u8;4])->Result<SocketAddr, CongestionHandlerError>{
        let peer_data = match self.pending_packet_ids.contains(packet_id){
            true => self.id_to_peer_map.get(packet_id).unwrap(),
            false => return Err(CongestionHandlerError::NoPacketWithIdError),
        };

        Ok(peer_data.clone())

    }

    pub fn search_id(&self, packet: &Packet)->Result<SocketAddr, CongestionHandlerError>{
        let id = packet.get_id();
        let peer_data = match self.pending_packet_ids.contains(id){
            true => self.id_to_peer_map.get(id).unwrap(),
            false => return Err(CongestionHandlerError::NoPacketWithIdError),
        };

        Ok(peer_data.clone())
    }
}

