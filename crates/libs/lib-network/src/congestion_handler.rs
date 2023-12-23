use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};

use std::sync::{Mutex, Arc, PoisonError, Condvar, MutexGuard};
use std::collections::VecDeque;

use lib_web::discovery::Peer;
// use crate::{peer_data::*, packet};
use crate::packet::*;
use crate::action::*;

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
    fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(Self{ packets_to_treat: VecDeque::new() }))
    }

    pub fn lock_and_push(queue: Arc<Mutex<ReceiveQueue>>,
                     packet: Packet, sock_addr: SocketAddr){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_packet(sock_addr, packet);

    }

    pub fn lock_and_pop(queue: Arc<Mutex<ReceiveQueue>>)->Option<(Packet, SocketAddr)>{
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.pop_packet()
    }

    pub fn push_packet(&mut self, sock_addr: SocketAddr, packet: Packet){
        self.packets_to_treat.push_back((packet, sock_addr));
    }

    pub fn pop_packet(&mut self)->Option<(Packet, SocketAddr)>{
        self.packets_to_treat.pop_front()
    }

    pub fn is_empty(&self)->bool{
        self.packets_to_treat.is_empty()
    }
}

pub struct SendQueue {
    packets_to_send: VecDeque<(Packet, SocketAddr)>,
}
impl SendQueue {
    fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(Self{ packets_to_send: VecDeque::new() }))
    }

    pub fn lock_and_push(queue: Arc<Mutex<SendQueue>>,
                     packet: Packet, sock_addr: SocketAddr){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_packet(packet, sock_addr)

    }

    pub fn lock_and_pop(queue: Arc<Mutex<SendQueue>>){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
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

    pub fn get_packet(&mut self)->Option<(Packet, SocketAddr)> {
        let packet_for_addr = match self.packets_to_send.front() {
            Some((packet, sock_addr)) =>{
                Some((packet.clone(), sock_addr.clone()))
            },
            None=> None,
        };
        self.pop_packet();
        packet_for_addr
    }

    pub fn is_empty(&self)->bool{
        self.packets_to_send.is_empty()
    }
}

pub struct QueueState {
    is_not_empty: (Mutex<bool>, Condvar),
}
impl QueueState {
    pub fn build_arc()->Arc<Self>{
        Arc::new(Self{ is_not_empty: (Mutex::new(false), Condvar::new()) })
    }
    pub fn wait(&self){
        /*
            Get the lock once and give it to a Condvar.
            The wait method atomically unlocks the mutex and waits
            for a notification.
        */
        let (state_lock, notif_var) = &self.is_not_empty;
        let mut start_or_wait = state_lock.lock().unwrap();

        /*
            Due to some obscure reasons the Condvar is 
            susceptible to spurious wakeups so that we
            need to pair it with a variable change on the Mutex
        */
        while !*start_or_wait {
            start_or_wait = notif_var.wait(start_or_wait).unwrap();
        }
    }

    pub fn set_empty_queue(queue_state: Arc<QueueState>){
        let (state_lock, _) = &queue_state.is_not_empty;
        let mut state_guard = match state_lock.lock(){
            Ok(state_guard)=> state_guard,
            Err(poison_error)=>
                    panic!("QueueState poisoned, sender panicked ?"),
        };

        *state_guard = false;
    }

    pub fn set_non_empty_queue(queue_state: Arc<QueueState>){
        /*
            Put the lock state to true and get the notifyer to 
            tell the other threads to wake up
        */
        let (state_lock, notifyer) = &queue_state.is_not_empty;
        let mut state_guard = match state_lock.lock(){
            Ok(state_guard)=> state_guard,
            Err(poison_error)=>
                    panic!("QueueState poisoned, sender panicked ?"),
        };

        *state_guard = true;
        notifyer.notify_all();
    }
}

pub struct ActionQueue{
    actions: VecDeque<Action>,
}

impl ActionQueue{
    fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(Self{ actions: VecDeque::new() }))
    }

    pub fn lock_and_push(queue: Arc<Mutex<ActionQueue>>,
                     action: Action){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_action(action);
    }

    pub fn lock_and_pop(queue: Arc<Mutex<ActionQueue>>)->Option<Action>{
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.pop_action()

    }
    pub fn push_action(&mut self, action: Action){
        self.actions.push_back(action);
    }

    pub fn pop_action(&mut self)->Option<Action>{
        self.actions.pop_front()
    }

    pub fn get_action(&mut self)->Option<Action> {
        let mut front_action = self.actions.front().clone();
        let front_action = match front_action{
            Some(action)=> Some(action.clone()),
            None=> None,
        };
        self.pop_action();
        front_action
    }

    pub fn is_empty(&self)->bool{
        self.actions.is_empty()
    }

}


// pub struct ActivePeers {
//     peers: Vec<PeerData>,
//     addr_map: HashMap<SocketAddr, PeerData>,
// }

// impl ActivePeers {

//     fn build_mutex()->Arc<Mutex<Self>>{
//         Arc::new(Mutex::new(Self{ peers: vec![], addr_map: HashMap::new() }))
//     }

//     fn add_peer(&mut self, peer: PeerData){

//     }

//     fn pop_peer(&mut self, peer: PeerData){
//         // let sock_addr_ind = &mut self.peers
//         //                         .iter().position(
//         //                             |peer_data| peer_data.eq(&peer)
//         //                         );
//     }

// }



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
    pub fn lock_and_add_packet_id_raw(pending_ids: Arc<Mutex<PendingIds>>,
                                     id: &[u8;4], peer_addr: &SocketAddr){
        let mut pending_ids_guard = 
            match pending_ids.lock(){
                Ok(pending_ids_guard)=>
                             pending_ids_guard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        pending_ids_guard.id_to_addr.insert(id.clone(),  peer_addr.clone());
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

pub fn build_queues()->(Arc<Mutex<ReceiveQueue>>,
                        Arc<Mutex<SendQueue>>,
                        Arc<Mutex<PendingIds>>,
                        Arc<Mutex<ActionQueue>>,
                        Arc<QueueState>,
                        Arc<QueueState>,
                        Arc<QueueState>){

    (ReceiveQueue::build_mutex(),
     SendQueue::build_mutex(),
     PendingIds::build_mutex(),
     ActionQueue::build_mutex(),
     QueueState::build_arc(),
     QueueState::build_arc(),
     QueueState::build_arc(),
    )
}
