use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};

use std::sync::{Mutex, Arc, PoisonError};
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
    pub fn lock_and_push(queue: Arc<Mutex<ReceiveQueue>>,
                     packet: Packet, sock_addr: SocketAddr){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(PoisonError)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_packet(packet, sock_addr)

    }

    pub fn lock_and_pop(queue: Arc<Mutex<ReceiveQueue>>){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(PoisonError)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.pop_packet()

    }
    pub fn push_packet(&mut self, packet: Packet, sock_addr: SocketAddr){
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
    pub fn lock_and_push(queue: Arc<Mutex<SendQueue>>,
                     packet: Packet, sock_addr: SocketAddr){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(PoisonError)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_packet(packet, sock_addr)

    }

    pub fn lock_and_pop(queue: Arc<Mutex<SendQueue>>){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(PoisonError)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.pop_packet()

    }
    pub fn push_packet(&mut self, packet: Packet, sock_addr: SocketAddr){
        self.packets_to_send.push_back((packet, sock_addr));
    }

    pub fn pop_packet(&mut self){
        self.packets_to_send.pop_front();
    }

    pub fn get_packet(&self)->Option<(Packet, SocketAddr)> {
        match self.packets_to_send.front() {
            Some((packet, sock_addr)) => Some((packet.clone(), sock_addr.clone())),
            None=> None,
        }
    }

    pub fn is_empty(&self)->bool{
        self.packets_to_send.is_empty()
    }
}


// pub struct ActionQueue{
//     actions: VecDeque<Action>,
// }


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
    id_to_addr: HashMap<[u8;4], SocketAddr>,
}

impl PendingIds{
    pub fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(PendingIds::default()))
    }
    /*Each time a packet is sent, no access to raw packet so need Packet struct */
    pub fn add_packet_id_raw(&mut self, id: [u8;4], peer_addr: &SocketAddr){
        self.id_to_addr.insert(id.clone(),  peer_addr.clone());
    }


    /*Each time a packet is received, it is received as raw bytes so access to Id directly*/
    pub fn pop_packet_id(&mut self, packet_id: &[u8; 4]){
        self.id_to_addr.remove(packet_id);
    }   

    pub fn search_id_raw(&self, packet_id: &[u8;4])->Result<(SocketAddr), CongestionHandlerError>{
        let addr = self.id_to_addr.get(packet_id);

        match addr {
            Some(addr) => return Ok(addr.clone()),
            None => return Err(CongestionHandlerError::NoPacketWithIdError),
        };
    }

    pub fn search_id(&self, packet: &Packet)->Result<SocketAddr, CongestionHandlerError>{
        let id = packet.get_id();

        self.search_id_raw(id)
    }
}

