use std::collections::HashMap;
use std::net::SocketAddr;

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, RwLock};

use log::{info, error};
use std::time::{Duration, Instant};

// use crate::{peer_data::*, packet};
use crate::action::*;
use crate::packet::*;

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
    pub fn build_arc() -> Arc<Self> {
        Arc::new(Self {
            is_not_empty: (Mutex::new(false), Condvar::new()),
        })
    }
    pub fn wait(&self) {
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
    pub fn wait_timeout_ms(&self, timeout: u64) -> Result<(), CongestionHandlerError> {
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
            start_or_wait =
                match notif_var.wait_timeout(start_or_wait, Duration::from_millis(timeout)) {
                    Ok(result) => {
                        if result.1.timed_out() {
                            return Err(CongestionHandlerError::TimeOutError);
                        }
                        result.0
                    }
                    Err(_) => {
                        error!("poison in QueueState");
                        panic!("Poison")
                    }
                }
        }
        Ok(())
    }

    pub fn set_empty_queue(queue_state: Arc<QueueState>) {
        let (state_lock, _) = &queue_state.is_not_empty;
        let mut state_guard = match state_lock.lock() {
            Ok(state_guard) => state_guard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("QueueState poisoned, sender panicked ?")
            }
        };

        *state_guard = false;
    }

    pub fn set_non_empty_queue(queue_state: Arc<QueueState>) {
        /*
            Put the lock state to true and get the notifyer to
            tell the other threads to wake up
        */
        let (state_lock, notifyer) = &queue_state.is_not_empty;
        let mut state_guard = match state_lock.lock() {
            Ok(state_guard) => state_guard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("QueueState poisoned, sender panicked ?")
            }
        };

        *state_guard = true;
        notifyer.notify_all();
    }
}

pub struct Queue<T> {
    data: VecDeque<T>,
    // can_pop: bool,
}

impl<T: Clone> Queue<T> {
    fn build_mutex() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            data: VecDeque::new(),
            // can_pop: true,
        }))
    }
    fn build_rwlock() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            data: VecDeque::new(),
            // can_pop: false,
        }))
    }

    pub fn read_lock_and_peek(queue: Arc<RwLock<Queue<T>>>) -> Option<T> {
        let queue_guard = match queue.read() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };
        queue_guard.peek_front()
    }
    /*For privileged reader */
    pub fn write_lock_and_get(queue: Arc<RwLock<Queue<T>>>) -> Option<T> {
        let mut queue_guard = match queue.write() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };

        // queue_guard.can_pop = true;
        queue_guard.get_front()
    }

    pub fn write_lock_and_push(queue: Arc<RwLock<Queue<T>>>, data: T) {
        let mut queue_guard = match queue.write() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };

        /*Only the pusher can pop the data,
        some privileged reader can set can_pop to true */
        // if queue_guard.can_pop {
        //     queue_guard.pop_front();
        //     queue_guard.push_back(data);
        // } else {
        queue_guard.push_back(data);
        // }
    }
    pub fn lock_and_push(queue: Arc<Mutex<Queue<T>>>, data: T) {
        let mut queue_guard = match queue.lock() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };

        queue_guard.push_back(data);
    }
    pub fn lock_and_push_mul(queue: Arc<Mutex<Queue<T>>>, data_vec: Vec<T>) {
        let mut queue_guard = match queue.lock() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };

        queue_guard.append_back(data_vec);
    }

    /*Not used */
    pub fn write_lock_and_pop_if_can(queue: Arc<RwLock<Queue<T>>>) -> Option<T> {
        let mut queue_guard = match queue.write() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };

        // if queue_guard.can_pop {
        queue_guard.pop_front()
        // } else {
        //     queue_guard.peek_front()
        // }
    }
    pub fn lock_and_pop(queue: Arc<Mutex<Queue<T>>>) -> Option<T> {
        let mut queue_guard = match queue.lock() {
            Ok(queue_gard) => queue_gard,
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Mutex is poisoned, some thread panicked")
            }
        };

        queue_guard.pop_front()
    }
    pub fn push_back(&mut self, data: T) {
        self.data.push_back(data);
    }
    pub fn append_back(&mut self, data_vec: impl Into<VecDeque<T>>) {
        self.data.extend(data_vec.into());
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.data.pop_front()
    }
    pub fn flush_and_front(&mut self) -> Option<T> {
        let front = self.data.front();
        let front = match front {
            Some(front) => Some(front.clone()),
            None => None,
        };
        self.data.clear();
        front
    }

    pub fn peek_front(&self) -> Option<T> {
        let front = self.data.front();
        let front = match front {
            Some(front) => Some(front.clone()),
            None => None,
        };
        front
    }
    pub fn get_front(&mut self) -> Option<T> {
        let front = self.data.front().cloned();
        let front = match front {
            Some(front) => Some(front.clone()),
            None => None,
        };
        self.pop_front();
        front
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[derive(Debug)]
pub enum CongestionHandlerError {
    NoPeerWithAddrError,
    NoPacketWithIdError,
    AddrAndIdDontMatchError,
    TimeOutError,
}

impl std::fmt::Display for CongestionHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CongestionHandlerError::NoPacketWithIdError => {
                write!(f, "No pending response for given Id")
            }
            CongestionHandlerError::NoPeerWithAddrError => {
                write!(f, "No peer with given socket address")
            }
            CongestionHandlerError::AddrAndIdDontMatchError => {
                write!(f, "Id and address from packet don't match")
            }
            CongestionHandlerError::TimeOutError => {
                write!(f, "Wait timed out")
            }
        }
    }
}

/*
    Tracks the ids of the packet sent. The protocol
    asserts that a response to a packet must have the
    same id as the packet it responds to.
*/
#[derive(Default)]
pub struct PendingIds {
    id_to_addr: HashMap<
        [u8; 4],
        (
            /*addr to which it was sent */
            SocketAddr,
            /*for convenience */
            PacketType,
            /*timeout timer */
            Instant,
            /*resending attempts */
            usize,
            // /*Attempted NatTraversal */
            bool,
        ),
    >,
    id_to_packet: HashMap<[u8; 4], (Packet, SocketAddr)>, //attempted_nat_trav: HashMap<SocketAddr, bool>?
}

impl PendingIds {
    pub fn build_mutex() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(PendingIds::default()))
    }

    // /*Flushes the ids every 0.1 s, so that the resend task has
    // time to check the unanswered ids and resend them */
    // pub fn launch_flusher_task(pending_ids: Arc<Mutex<PendingIds>>) {
    //     tokio::spawn(async move {
    //         loop {
    //             sleep(Duration::from_millis(100)).await;
    //             let mut guard = match pending_ids.lock() {
    //                 Ok(guard) => guard,
    //                 Err(poison_error) => break,
    //             };

    //             let mut to_pop = vec![];

    //             for id in guard.id_to_addr.keys() {
    //                 let (_, _, attempts, attempted_nat_trav, can_pop_key) =
    //                     guard.id_to_addr.get(id).unwrap();
    //                 if (*can_pop_key == false) && (*attempted_nat_trav == true) {
    //                     to_pop.push(*id);
    //                 }
    //             }
    //             for id in to_pop {
    //                 guard.pop_packet_id(&id);
    //             }
    //         }
    //     });
    // }

    /*
        This function checks if the packet about it be sent has already
        been sent before. If it is the case, increments the attempt number
        associated to the id. Also checks if a NAT traversal has been attempted
        with the given socket address.
    */

    pub fn lock_and_add_id(
        pending_ids: Arc<Mutex<PendingIds>>,
        packet: &Packet,
        peer_addr: &SocketAddr,
    ) {
        if packet.is(*&PacketType::Error) {
            return;
        }else if packet.is(*&PacketType::NatTraversalRequest){
            return;
        } else if packet.is_response() {
            return;
        }

        let mut pending_ids_guard = match pending_ids.lock() {
            Ok(guard) => guard,
            /*If Mutex is poisoned stop every thread, something is wrong */
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Poisoned IDs mutex")
            }
        };

        /*
            Check if id exists before adding, doesn't check timer.
            If a packet is in queue to resend. It means the timer
            (RTO) has been checked.
        */
        match pending_ids_guard.search_id_mut(&packet) {
            /*id exists */
            Ok((sock_addr, _packet_type, _, attempts)) => {
                if *sock_addr != *peer_addr {
                    return;
                } else {
                    *attempts += 1;
                    return;
                }
            }
            Err(CongestionHandlerError::NoPacketWithIdError) => {
                pending_ids_guard.id_to_addr.insert(
                    *packet.get_id(),
                    (
                        *peer_addr,
                        *packet.get_packet_type(),
                        Instant::now(),
                        0,
                        false,
                    ),
                );
                pending_ids_guard
                    .id_to_packet
                    .insert(*packet.get_id(), (packet.clone(), *peer_addr));
                return;
            }
            Err(e) => {
                error!("{e}");
                panic!("Should not happen, handle packet")
            }
        };
    }

    /*
       This function is used to determine if
       a packet is a response or a request.
    */
    pub fn id_exists(
        pending_ids: Arc<Mutex<PendingIds>>,
        packet: &Packet,
        socket_addr: SocketAddr,
    ) -> Result<SocketAddr, CongestionHandlerError> {
        if *packet.get_packet_type() == PacketType::NatTraversal {
            return Ok(socket_addr);
        }
        /*get mutex to check and pop id if it is a response */
        let mut pending_ids_guard = match pending_ids.lock() {
            Ok(guard) => guard,
            /*If Mutex is poisoned stop every thread, something is wrong */
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Poisoned IDs mutex")
            }
        };

        /*Check if id exists */
        match pending_ids_guard.search_id(&packet) {
            Ok((sock_addr, ..)) => {
                /*if id exists, pop the packet before handling it. */
                /*We choose to not handle the collisions. */
                /*compute rto ? */
                pending_ids_guard.pop_packet_id(packet.get_id());

                /*Now check if the address it was sent to is
                the same as the address it was received from */
                let addr_matches_id = {
                    let addr_match_id;

                    if sock_addr != socket_addr {
                        addr_match_id = Err(CongestionHandlerError::AddrAndIdDontMatchError);
                    } else {
                        addr_match_id = Ok(sock_addr);
                    }
                    addr_match_id
                };
                addr_matches_id
            }
            Err(CongestionHandlerError::NoPacketWithIdError) => {
                Err(CongestionHandlerError::NoPacketWithIdError)
            }
            Err(e) => {
                error!("{e}");
                panic!("Should not happen, handle packet")
            }
        }
        /*Mutex is dropped here */
    }

    /*Because hashmaps are not made to be enumerated, this may be heavy.*/
    /*Should ignore nat traversal requests */
    pub fn packets_to_resend(
        pending_ids: Arc<Mutex<PendingIds>>,
    ) -> (Vec<SocketAddr>, Vec<(Packet, SocketAddr)>) {
        let mut pending_ids_guard = match pending_ids.lock() {
            Ok(guard) => guard,
            /*If Mutex is poisoned stop every thread, something is wrong */
            Err(poison_error) => {
                error!("{poison_error}");
                panic!("Poisoned IDs mutex")
            }
        };

        let mut id_to_resend = vec![];
        let mut id_to_send_nat_trav = vec![];
        let mut id_to_pop = vec![];

        for (id, (_addr, packet_type, _, attempts, _nat_trav)) in
            pending_ids_guard.id_to_addr.iter_mut()
        {
            match *attempts {
                3 | 5 | 7 | 9=>{
                    *attempts+=1;
                    id_to_send_nat_trav.push(*id);
                    id_to_resend.push(*id);
                },
                10.. => {
                    id_to_pop.push(*id);
                }
                _ => match packet_type {
                    PacketType::NatTraversalRequest => id_to_pop.push(*id),
                    _ => {
                        *attempts += 1;
                        id_to_resend.push(*id);
                    }
                },
            };
        }
        let mut addr_to_send_nat_trav = vec![];
        /*WEIRD*/
        // debug!(
        //     "to nat trav {:?}\nto pop {:?}\n to resend {:?}",
        //     id_to_send_nat_trav, id_to_pop, id_to_resend
        // );

        for id in id_to_pop {
            info!("Packet with id {id:?} timed out");
            pending_ids_guard.id_to_packet.remove(&id);
            pending_ids_guard.id_to_addr.remove(&id).unwrap();
        }

        let mut packet_to_resend = vec![];
        for id in &id_to_resend {
            let packet_for_addr = pending_ids_guard.id_to_packet.get(id).unwrap().clone();
            addr_to_send_nat_trav.push(*&packet_for_addr.1);
            packet_to_resend.push(packet_for_addr);
        }

        // println!("packet to resend {:?}", packet_to_resend);

        (addr_to_send_nat_trav, packet_to_resend)
    }
    pub fn pop_packet_id(&mut self, packet_id: &[u8; 4]) {
        self.id_to_addr.remove(packet_id);
    }

    pub fn search_id_raw_mut(
        &mut self,
        packet_id: &[u8; 4],
    ) -> Result<(&mut SocketAddr, &mut PacketType, &mut Instant, &mut usize), CongestionHandlerError>
    {
        let addr = self.id_to_addr.get_mut(packet_id);

        match addr {
            Some((sock_addr, packet_type, timer, attempts, _)) => {
                return Ok((sock_addr, packet_type, timer, attempts))
            }
            None => return Err(CongestionHandlerError::NoPacketWithIdError),
        };
    }

    pub fn search_id_raw(
        &self,
        packet_id: &[u8; 4],
    ) -> Result<(SocketAddr, usize), CongestionHandlerError> {
        let addr = self.id_to_addr.get(packet_id);

        match addr {
            Some((sock_addr, _, _, attempts, _)) => return Ok((*sock_addr, *attempts)),
            None => return Err(CongestionHandlerError::NoPacketWithIdError),
        };
    }

    pub fn search_id_mut(
        &mut self,
        packet: &Packet,
    ) -> Result<(&mut SocketAddr, &mut PacketType, &mut Instant, &mut usize), CongestionHandlerError>
    {
        let id = packet.get_id();

        self.search_id_raw_mut(id)
    }
    pub fn search_id(
        &self,
        packet: &Packet,
    ) -> Result<(SocketAddr, usize), CongestionHandlerError> {
        let id = packet.get_id();

        self.search_id_raw(id)
    }
}

pub fn build_queues() -> (
    Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    Arc<Mutex<Queue<Action>>>,
    Arc<RwLock<Queue<Action>>>,
    Arc<Mutex<PendingIds>>,
    Arc<QueueState>,
    Arc<QueueState>,
    Arc<QueueState>,
    Arc<QueueState>,
    Arc<QueueState>,
) {
    (
        Queue::build_mutex(),
        Queue::build_mutex(),
        Queue::build_mutex(),
        Queue::build_rwlock(),
        PendingIds::build_mutex(),
        QueueState::build_arc(),
        QueueState::build_arc(),
        QueueState::build_arc(),
        QueueState::build_arc(),
        QueueState::build_arc(),
    )
}
