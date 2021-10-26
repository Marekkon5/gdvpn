use std::io::Cursor;
use std::time::Duration;
use async_channel::{bounded, Receiver, Sender};

use crate::gdrive::GDrive;

pub struct PacketQueue {
    rx: Receiver<Response>,
    tx: Sender<Message>
}

impl PacketQueue {
    /// Start a background queue process
    /// files = Vec<google drive file id>
    pub async fn new(
        packet_duration: Duration, 
        max_packet_queue: usize, 
        gdrive: GDrive, 
        files: Vec<String>,
        parent_folder: String,
    ) -> PacketQueue {
        let (tx_0, rx_0) = bounded(max_packet_queue);
        let (tx_1, rx_1) = bounded(max_packet_queue);
        tokio::task::spawn(async move {
            let mut q = PacketQueueInner {
                delay: packet_duration,
                gdrive, files, parent_folder,
                rx: rx_1,
                tx: tx_0,
                file_index: 0,
            };
            q.worker_thread().await
        });

        PacketQueue {
            rx: rx_0,
            tx: tx_1
        }
    }

    /// Enqueue packet
    pub async fn queue_packet(&self, packet: Vec<u8>) {
        self.tx.send(Message::Packet(packet)).await.unwrap();
    }

    /// Receive response
    pub async fn recv(&self) -> Response {
        self.rx.recv().await.unwrap()
    }
}

enum Message {
    Packet(Vec<u8>)
}

pub enum Response {
    /// Affected file ID
    FileIndex(usize),
}

pub struct PacketQueueInner {
    delay: Duration,
    gdrive: GDrive,
    rx: Receiver<Message>,
    tx: Sender<Response>,
    files: Vec<String>,
    file_index: usize,
    parent_folder: String,
}

impl PacketQueueInner {
    /// Runs in task
    async fn worker_thread(&mut self) {
        loop { 
            // Receive queue until duration expires
            let mut queue = vec![];
            let mut timeout_duration = self.delay.clone();
            let mut last_request = timestamp!();
            while let Ok(Ok(data)) = tokio::time::timeout(timeout_duration, self.rx.recv()).await {
                let Message::Packet(data) = data;
                queue.push(data);
                
                // Calculate new timeout
                let took = Duration::from_millis((timestamp!() - last_request) as u64);
                if took > timeout_duration {
                    break;
                }
                timeout_duration -= took;
                last_request = timestamp!();
            }
            if queue.is_empty() {
                continue;
            }

            // Create buffer
            let mut buffer = vec![];
            for packet in queue {
                buffer.push((packet.len() as i16).to_be_bytes().to_vec());
                buffer.push(packet);
            }
            let buffer = buffer.concat();

            debug!("Uploading: {}KB", buffer.len() / 1024);
            // Upload
            let file_id = &self.files[self.file_index];
            match self.gdrive.upload(
                Cursor::new(buffer), 
                &self.file_index.to_string(), 
                &self.parent_folder, 
                Some(file_id)
            ).await {
                Ok(uploaded) => {
                    info!("Uploaded: ID: {}, Index: {}", uploaded.id, self.file_index);
                    self.tx.send(Response::FileIndex(self.file_index)).await.unwrap();
                },
                Err(e) => {
                    error!("Failed uploading, dropping packets: {}", e);
                }
            }
            
            self.file_index += 1;
            if self.file_index == self.files.len() {
                self.file_index = 0;
            }
        }
    }
}
