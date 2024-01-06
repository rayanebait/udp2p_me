use crate::{action::Action, peer::Peer};
use crate::{
    congestion_handler::{PendingIds, Queue, QueueState},
    import_export::keep_alive_to_peer,
    packet::{Packet, PacketBuilder},
    peer::ActivePeers,
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

/*Remark: Packets/ids are popped only when received. */
pub fn resend_task(
    pending_ids: Arc<Mutex<PendingIds>>,
    _peek_active_peers: Arc<Mutex<ActivePeers>>,
    action_queue: Arc<Mutex<Queue<Action>>>,
    action_queue_state: Arc<QueueState>,
    sending_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    sending_queue_state: Arc<QueueState>,
    my_data: Arc<Peer>,
) {
    tokio::spawn(async move {
        /*Shouldn't wait/resend after replies!!  */
        let server_socket_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        let server_socket_addr6: SocketAddr =
            "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();

        keep_alive_to_peer(
            action_queue.clone(),
            action_queue_state.clone(),
            server_socket_addr4.clone(),
            my_data,
            30_000,
        );
        loop {
            sleep(Duration::from_secs(2));
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
                nat_trav_packets.push((
                    PacketBuilder::nat_traversal_request_from_addr_packet(addr),
                    server_socket_addr6,
                ));
            }
            Queue::lock_and_push_mul(sending_queue.clone(), nat_trav_packets);
        }
    });
}
