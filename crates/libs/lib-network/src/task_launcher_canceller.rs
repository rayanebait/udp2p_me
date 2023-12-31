use tokio::net::UdpSocket;

use crate::resend::resend_task;

use {
    crate::{
        action::Action,
        congestion_handler::*,
        handle_action::handle_action_task,
        handle_packet::handle_packet_task,
        packet::Packet,
        peer::{ActivePeers, Peer},
        process::process_task,
        sender_receiver::{receiver4, receiver6, sender},
    },
    std::{
        net::SocketAddr,
        sync::{Arc, Mutex, RwLock},
    },
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
    my_data_own: Peer,
    sock4: Arc<UdpSocket>,
    sock6: Arc<UdpSocket>,
    exporting: bool,
    path: std::path::PathBuf
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
        receiver4(
            Arc::clone(&sock4),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
        );

        receiver6(
            Arc::clone(&sock6),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
        );

        handle_packet_task(
            Arc::clone(&pending_ids),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
        );
        handle_action_task(
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),

        );
        process_task(
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&active_peers),
            my_data_own,
            exporting,
            path
        );

        sender(
            Arc::clone(&sock4),
            Arc::clone(&sock6),
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&pending_ids),
        );
        resend_task(
            pending_ids.clone(),
            active_peers.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            send_queue.clone(),
            send_queue_state.clone(),
            my_data.clone()
        );

    });
}
