use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};

use std::sync::{Mutex, Arc};
use std::collections::VecDeque;

use lib_web::discovery::Peer;
use crate::{peer_data::*, packet};
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

pub struct ReceiveQueue {
    packets_to_treat: VecDeque<(Packet, SocketAddr)>,

}

impl ReceiveQueue {
    pub fn add_packet(&mut self, packet: Packet, sock_addr: SocketAddr){
        self.packets_to_treat.push_back((packet, sock_addr));
    }

    pub fn pop_packet(&mut self){
        self.packets_to_treat.pop_front();
    }

    pub fn is_empty(&self)->bool{
        self.packets_to_treat.is_empty()
    }
}

pub struct SendQueue {
    packets_to_send: VecDeque<(Packet, SocketAddr)>,
}

impl SendQueue {
    pub fn add_packet(&mut self, packet: Packet, sock_addr: SocketAddr){
        self.packets_to_send.push_back((packet, sock_addr));
    }

    pub fn pop_packet(&mut self){
        self.packets_to_send.pop_front();
    }

    pub fn is_empty(&self)->bool{
        self.packets_to_send.is_empty()
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



#[derive(Debug)]
pub enum CongestionHandlerError {
    NoPeerWithAddrError,
    NoPacketWithIdError,
    AddrAndIdDontMatchError,
}

impl std::fmt::Display for CongestionHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CongestionHandlerError::NoPacketWithIdError=>
                         write!(f, "No pending response for given Id"),
            CongestionHandlerError::NoPeerWithAddrError=>
                         write!(f, "No peer with given socket address"),
            CongestionHandlerError::AddrAndIdDontMatchError=>
                         write!(f, "Id and address from packet don't match"),
        }        
    }
}

#[derive(Default)]
pub struct PendingIds{
    pending_packet_ids: Vec<[u8;4]>,
    id_to_index: HashMap<[u8;4], (usize, SocketAddr)>,
    nb_ids: usize,
}

impl PendingIds{
    pub fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(PendingIds::default()))
    }
    /*Each time a packet is sent, no access to raw packet so need Packet struct */
    pub fn add_packet_id_raw(&mut self, id: [u8;4], peer_addr: &SocketAddr){
        
        self.pending_packet_ids.push(id.clone());

        self.id_to_index.insert(id.clone(), (self.nb_ids, peer_addr.clone()));
        self.nb_ids+=1;
    }


    /*Each time a packet is received, it is received as raw bytes so access to Id directly*/
    pub fn pop_packet_id(&mut self, packet_id: &[u8; 4]){
        let entry = self.id_to_index.get(packet_id);
    
        match entry {
            Some((ind, sock_addr)) => {
                self.pending_packet_ids.remove(*ind);
                self.id_to_index.remove(packet_id);
                if self.nb_ids > 0 {
                    self.nb_ids-=1;
                }
                return;
            },
            /*Simply return if Id doesn't match any */
            None => return,
        };
    }   

    pub fn search_id_raw(&self, packet_id: &[u8;4])->Result<(SocketAddr), CongestionHandlerError>{
        let id_ind = self.id_to_index.get(packet_id);

        match id_ind {
            Some((ind, sock_addr)) => return Ok(sock_addr.clone()),
            None => return Err(CongestionHandlerError::NoPacketWithIdError),
        };
    }

    pub fn search_id(&self, packet: &Packet)->Result<SocketAddr, CongestionHandlerError>{
        let id = packet.get_id();

        self.search_id_raw(id)
    }
}

