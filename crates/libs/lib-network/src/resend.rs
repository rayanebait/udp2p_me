

use tokio_util::sync::CancellationToken;
use tokio::select;
use log::debug;
use crate::{
    congestion_handler::{PendingIds, Queue, QueueState},
    packet::{Packet, PacketBuilder},
    peer::ActivePeers,
};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::sleep;


/*Remark: Packets/ids are popped only when received. */
pub fn resend_task(
    pending_ids: Arc<Mutex<PendingIds>>,
    _peek_active_peers: Arc<Mutex<ActivePeers>>,
    sending_queue: Arc<Mutex<Queue<(Packet, SocketAddr)>>>,
    sending_queue_state: Arc<QueueState>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        /*Shouldn't wait/resend after replies!!  */
        let server_socket_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        let server_socket_addr6: SocketAddr =
            "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        let non_ending_loop = async {

            loop {
                sleep(Duration::from_millis(1000)).await;
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
        };

        select!{
            _= non_ending_loop =>(),
            _= cancel.cancelled()=>{
                debug!("resend cancelled")
            },
        }
    });
}
