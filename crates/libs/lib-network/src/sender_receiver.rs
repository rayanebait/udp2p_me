use log::{debug, error};
use tokio_util::sync::CancellationToken;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::{
    net::UdpSocket,
    select
};


use crate::congestion_handler::{PendingIds, Queue, QueueState};
use crate::packet::Packet;

/*
    Maybe lock only first and last element ? sender accesses only the first
    handle action accesses only the last
*/
pub fn receiver4(
    sock4: Arc<UdpSocket>,
    receive_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    receive_queue_state: Arc<QueueState>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            /*Get the first packet in the Queue or None if the queue
            is empty. Also break, if task is cancelled. */
            let (sock_addr, packet) = 
                match select!{
                    packet_and_addr = Packet::recv_from(&sock4)=> {
                        packet_and_addr
                    },
                    
                    _ = cancel.cancelled()=>{
                        debug!("receiver cancelled");
                        break
                    }
                }{
                    Ok(packet_and_addr)=> packet_and_addr,
                    _=> continue,
                };

            debug!(
                "Received {} packet from {}\n",
                packet.get_packet_type(),
                sock_addr,
            );

            Queue::lock_and_push(Arc::clone(&receive_queue), (packet, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&receive_queue_state));
        }
    });
}

pub fn receiver6(
    sock6: Arc<UdpSocket>,
    receive_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    receive_queue_state: Arc<QueueState>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            /*Get the first packet in the Queue or None if the queue
            is empty */
            let (sock_addr, packet) = 
                match select!{
                    packet_and_addr = Packet::recv_from(&sock6)=> {
                        packet_and_addr
                    },
                    
                    _ = cancel.cancelled()=>{
                        break
                        debug!("received cancelled");
                    },
                }{
                    Ok(packet_and_addr)=> packet_and_addr,
                    _=> continue,
                };

            debug!(
                "Received {} packet from {} : {:?}\n",
                packet.get_packet_type(),
                sock_addr,
                packet
            );
            /*Maybe should be async? */
            Queue::lock_and_push(Arc::clone(&receive_queue), (packet, sock_addr));

            QueueState::set_non_empty_queue(Arc::clone(&receive_queue_state));
        }
    });
}

pub fn sender(
    sock4: Arc<UdpSocket>,
    sock6: Arc<UdpSocket>,
    send_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    send_queue_state: Arc<QueueState>,
    pending_ids_to_add: Arc<Mutex<PendingIds>>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        loop {
            /*Get the first packet in the Queue or None if the queue
            is empty */
            let packet_for_addr = {
                let mut guard = match send_queue.lock() {
                    Ok(guard) => guard,
                    Err(poison_error) => {
                        error!("{poison_error}");
                        panic!("Poisoned Mutex found in sender")
                    }
                };

                /*pops the front packet if it exists */
                guard.get_front()
            };

            /*If the queue is empty, put the thread to sleep until queue
            is not empty*/
            let (packet, sock_addr) = match packet_for_addr {
                Some(packet_for_addr) => packet_for_addr,
                None => {
                    QueueState::set_empty_queue(Arc::clone(&send_queue_state));
                    /*Passively wait for non empty queue state */
                    select! {
                        _ = async {
                            debug!("sender wait");
                            send_queue_state.wait()
                        }=>{
                            continue
                        }
                        _ = cancel.cancelled()=>{
                            debug!("sender cancelled");
                            break
                        }
                    }
                }
            };

            PendingIds::lock_and_add_id(Arc::clone(&pending_ids_to_add), &packet, &sock_addr);
            let sent = match sock_addr {
                SocketAddr::V4(_) => packet.send_to_addr(&sock4, &sock_addr).await,
                SocketAddr::V6(_) => packet.send_to_addr(&sock6, &sock_addr).await,
            };

            match sent {
                Ok(_size) => {
                    debug!(
                        "Sending {} packet to {}\n",
                        packet.get_packet_type(),
                        sock_addr
                    );
                }
                Err(e) => {
                    error!("{}", e)
                }
            }
        }
    });
}
