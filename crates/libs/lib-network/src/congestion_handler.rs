use std::collections::HashMap;
use std::net::{UdpSocket, SocketAddr};

use std::sync::{Mutex, Arc, PoisonError, Condvar, MutexGuard, RwLock};
use std::collections::VecDeque;

// use crate::{peer_data::*, packet};
use crate::packet::{*, self};
use crate::action::*;

#[derive(Default)]
// pub struct ActiveSockets{
//     sockets: Vec<SocketAddr>,
// }

// impl ActiveSockets {
//     fn build_mutex()->Arc<Mutex<Self>>{
//         Arc::new(Mutex::new(Self{ sockets: vec![] }))
//     }

//     fn add_addr(&mut self, sock_addr: SocketAddr){

//     }

//     fn pop_addr(&mut self, sock_addr: SocketAddr){
//         let sock_addr_ind = &mut self.sockets
//                                 .iter().position(
//                                     |addr| addr.eq(&sock_addr)
//                                 );
//     }
// }

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


pub struct Queue<T> {
    data: VecDeque<T>,
}

impl<T: Clone> Queue<T>{
    fn build_mutex()->Arc<Mutex<Self>>{
        Arc::new(Mutex::new(Self{ data: VecDeque::new() }))
    }
    fn build_rwlock()->Arc<RwLock<Self>>{
        Arc::new(RwLock::new(Self{ data: VecDeque::new() }))
    }

    pub fn read_lock_and_peek(queue: Arc<RwLock<Queue<T>>>)->Option<T>{
        let mut queue_guard = 
            match queue.read(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.peek_front()
    }
    pub fn write_lock_and_push(queue: Arc<RwLock<Queue<T>>>,
                     data: T){
        let mut queue_guard = 
            match queue.write(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_back(data);
    }
    pub fn lock_and_push(queue: Arc<Mutex<Queue<T>>>,
                     data: T){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.push_back(data);
    }
    pub fn lock_and_push_mul(queue: Arc<Mutex<Queue<T>>>,
                    data_vec: Vec<T>){
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.append_back(data_vec);
}

    pub fn write_lock_and_pop(queue: Arc<RwLock<Queue<T>>>)->Option<T>{
        let mut queue_guard = 
            match queue.write(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.pop_front()
    }
    pub fn lock_and_pop(queue: Arc<Mutex<Queue<T>>>)->Option<T>{
        let mut queue_guard = 
            match queue.lock(){
                Ok(queue_gard)=>
                             queue_gard,
                Err(poison_error)=>
                             panic!("Mutex is poisoned, some thread panicked"),
            };
        
        queue_guard.pop_front()
    }
    pub fn push_back(&mut self, data: T){
        self.data.push_back(data);
    }
    pub fn append_back(&mut self, data_vec: impl Into<VecDeque<T>>){
        self.data.extend(data_vec.into());
    }

    pub fn pop_front(&mut self)->Option<T>{
        self.data.pop_front()
    }

    pub fn peek_front(&self)->Option<T> {
        let mut front = self.data.front();
        let front = match front{
            Some(front)=> Some(front.clone()),
            None=> None,
        };
        front
    }
    pub fn get_front(&mut self)->Option<T> {
        let mut front = self.data.front().cloned();
        let front = match front{
            Some(front)=> Some(front.clone()),
            None=> None,
        };
        self.pop_front();
        front
    }

    pub fn is_empty(&self)->bool{
        self.data.is_empty()
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

pub fn build_queues()->(Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
                        Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
                        Arc<Mutex<Queue<Action>>>,
                        Arc<RwLock<Queue<Action>>>,
                        Arc<Mutex<PendingIds>>,
                        Arc<QueueState>,
                        Arc<QueueState>,
                        Arc<QueueState>,
                        Arc<QueueState>
                    ){

    (Queue::build_mutex(),
     Queue::build_mutex(),
     Queue::build_mutex(),
     Queue::build_rwlock(),
     PendingIds::build_mutex(),
     QueueState::build_arc(),
     QueueState::build_arc(),
     QueueState::build_arc(),
     QueueState::build_arc(),
    )
}
