use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError, Condvar};
use std::thread::sleep;
use tokio::{net::UdpSocket, spawn};
use tokio_util::task::TaskTracker;

use crate::congestion_handler::{Queue, QueueState, PendingIds};
use crate::packet::{Packet, PacketError};

/*
    Maybe lock only first and last element ? sender accesses only the first
    handle action accesses only the last 
*/
pub async fn receiver(sock: Arc<UdpSocket>,
         receive_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
         receive_queue_state: Arc<QueueState>){
    tokio::spawn(async move {
        loop {
            /*Get the first packet in the Queue or None if the queue 
            is empty */
            let (sock_addr, packet) =
            match Packet::recv_from(&sock).await{
                Ok(packet_and_addr) => packet_and_addr,
                _=> continue,
            };

            /*Maybe should be async? */
            Queue::lock_and_push(
                    Arc::clone(&receive_queue),
                        ( packet, sock_addr));

            QueueState::set_non_empty_queue(Arc::clone(&receive_queue_state));

        };
    }).await;
}

pub async fn sender(sock: Arc<UdpSocket>,
         send_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
         send_queue_state: Arc<QueueState>,
         pending_ids_to_add: Arc<Mutex<PendingIds>>){
    tokio::spawn(async move {
        loop {
            /*Get the first packet in the Queue or None if the queue 
            is empty */
            let packet_for_addr ={
                let mut guard  = match send_queue.lock() {
                    Ok(guard) => guard,
                    Err(poison_error) =>
                                    panic!("Poisoned Mutex found in sender"),
                };

                /*pops the front packet if it exists */
                guard.get_front()
            };

            /*If the queue is empty, put the thread to sleep until queue 
            is not empty*/
            let (packet, sock_addr) = match packet_for_addr {
                Some(packet_for_addr) => packet_for_addr,
                None=> {
                    println!("sender wait");
                    QueueState::set_empty_queue(Arc::clone(&send_queue_state));
                    send_queue_state.wait();
                    // sleep(std::time::Duration::from_secs(1));
                    continue
                },
            };

            PendingIds::lock_and_add_packet_id_raw(Arc::clone(&pending_ids_to_add),
                                 packet.get_id(), &sock_addr);
            packet.send_to_addr(&sock, &sock_addr).await.unwrap();
            // println!("Sending packet: {:?}\n", packet);
        };
    }).await;
}