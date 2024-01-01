use tokio::net::UdpSocket;

use crate::resend::resend_task;

use {
    crate::{
        action::Action,
        congestion_handler::*,
        handle_action::handle_action_task,
        handle_packet::handle_packet_task,
        packet::Packet,
        peer::peer::{ActivePeers, Peer},
        process::process_task,
        sender_receiver::{receiver, sender},
    },
    std::{
        net::SocketAddr,
        sync::{Arc, Mutex, RwLock},
    },
    tokio::{join, time},
};

pub fn task_launcher(
    queues: (
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
    ),
    active_peers: Arc<Mutex<ActivePeers>>,
    my_data: Arc<Peer>,
    sock: Arc<UdpSocket>,
) {
    let (
        receive_queue,
        send_queue,
        action_queue,
        process_queue,
        pending_ids,
        receive_queue_state,
        action_queue_state,
        send_queue_state,
        process_queue_state,
        process_queue_readers_state,
    ) = queues;

    let (
        receive_queue,
        send_queue,
        action_queue,
        process_queue,
        pending_ids,
        receive_queue_state,
        action_queue_state,
        send_queue_state,
        process_queue_state,
        process_queue_readers_state,
    ) = (
        Arc::clone(&receive_queue),
        Arc::clone(&send_queue),
        Arc::clone(&action_queue),
        Arc::clone(&process_queue),
        Arc::clone(&pending_ids),
        Arc::clone(&receive_queue_state),
        Arc::clone(&action_queue_state),
        Arc::clone(&send_queue_state),
        Arc::clone(&process_queue_state),
        Arc::clone(&process_queue_readers_state),
    );

    tokio::spawn(async move {
        let receiving = receiver(
            Arc::clone(&sock),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
        );

        let handling = handle_packet_task(
            Arc::clone(&pending_ids),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
        );
        let processing_two = handle_action_task(
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
        );
        let processing_one = process_task(
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&active_peers),
            Arc::clone(&my_data),
            // Arc::clone(&map)
        );

        let sending = sender(
            Arc::clone(&sock),
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&pending_ids),
        );
        let resending = resend_task(pending_ids, active_peers, send_queue, send_queue_state);
    });
}
