use tokio_util::time::DelayQueue;

use crate::{
    action::Action,
    congestion_handler::{self, PendingIds, Queue, QueueState},
    packet::{Packet, PacketBuilder},
    peer::ActivePeers,
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
    thread::sleep,
    time::Duration,
};

/*

*/
/*Remark: Packets/ids are popped only when received. */
pub fn resend_task(
    pending_ids: Arc<Mutex<PendingIds>>,
    // pending_ids_state: Arc<QueueState>,
    peek_active_peers: Arc<Mutex<ActivePeers>>,
    sending_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    sending_queue_state: Arc<QueueState>,
) {
    tokio::spawn(async move {
        /*Shouldn't wait/resend after replies!!  */
        let server_socket_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        let server_socket_addr6: SocketAddr =
            "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        loop {
            sleep(Duration::from_secs(1));
            // pending_ids_state.wait();
            let (addr_to_send_nat_trav, packet_to_resend) =
                PendingIds::packets_to_resend(Arc::clone(&pending_ids));
            Queue::lock_and_push_mul(sending_queue.clone(), packet_to_resend);
            QueueState::set_non_empty_queue(sending_queue_state.clone());

            let mut nat_trav_packets = vec![];
            for addr in addr_to_send_nat_trav {
                nat_trav_packets.push((
                    PacketBuilder::nat_traversal_request_from_addr_packet(addr),
                    server_socket_addr4,
                ));
            }
            Queue::lock_and_push_mul(sending_queue.clone(), nat_trav_packets);
        }
    });
}

// pub fn track_single_packet_task(
//     pending_ids: Arc<Mutex<PendingIds>>,
//     peek_active_peers: Arc<Mutex<ActivePeers>>,
//     action_queue: Arc<Mutex<Queue<Action>>>,
//     action_queue_state: Arc<QueueState>,
//     id: [u8;4]
// ){

// }
