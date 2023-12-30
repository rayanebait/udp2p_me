use crate::{packet::Packet, action::Action, congestion_handler::{self, Queue, QueueState}};
use std::{sync::{Arc, Mutex, RwLock}, os::unix::net::SocketAddr};


pub async fn resend_task(
    peek_pending_ids_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    action_queue: Arc<Mutex<Queue<Action>>>
){
    


}