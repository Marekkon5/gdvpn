#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;

use std::io::Cursor;
use std::time::Duration;
use anyhow::Error;
use futures::{FutureExt, pin_mut, select};
use gdrive::GDrive;
use queue::PacketQueue;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio_tun::TunBuilder;

use oauth::OAuth;
use settings::Settings;


// Get timestamp macro
macro_rules! timestamp {
    () => {
        std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis()
    };
}

mod oauth;
mod queue;
mod gdrive;
mod settings;

fn main() {
    std::env::set_var("RUST_LOG", "gdvpn_server=debug,info");
    std::env::set_var("RUST_BACKTRACE", "1");
    pretty_env_logger::init();

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed creating runtime!");
    runtime.block_on(async move {
        // Initialize
        let oauth = OAuth::new("secret.json", Some("token.json"), Some("https://www.googleapis.com/auth/drive"))
            .await.expect("Failed initializing OAuth");
        let settings = Settings::load("settings.json").await.expect("Failed loading settings!");
        let gdrive = GDrive::new(oauth);
        
        // Allocate files
        info!("Allocating GDrive files!");
        let files = gdrive.list_folder(&settings.folder_id).await.expect("Failed getting folder items!");
        for i in files.items.len()..settings.file_count as usize {
            info!("Creating file: {} / {}", i, settings.file_count);
            gdrive.upload(
                Cursor::new(&[0,1,2,3]), 
                &i.to_string(), 
                &settings.folder_id, 
                None
            ).await.expect("Failed uploading file!");
        }
        let files = gdrive.list_folder(&settings.folder_id).await.expect("Failed getting folder items!");
        let files = files.items.into_iter().map(|f| f.id).collect::<Vec<String>>();
        
        // Create queue
        let queue = PacketQueue::new(
            Duration::from_millis(settings.packet_duration),
            settings.max_queued_packets,
            gdrive,
            files.clone(),
            settings.folder_id.to_string(),
        ).await;

        // Start
        start_server(&settings, queue, files).await.expect("Failed running server!");
    });
}


/// Handle incoming connections
async fn start_server(settings: &Settings, queue: PacketQueue, files: Vec<String>) -> Result<(), Error> {
    let server = TcpListener::bind(format!("0.0.0.0:{}", settings.port)).await?;
    info!("Started server!");

    loop {
        let (mut stream, _) = server.accept().await?;
        info!("New connection: {}", stream.peer_addr()?);

        // Sync file list
        let buffer = files.join("\n");
        stream.write_all(&(buffer.len() as i32).to_be_bytes()).await?;
        stream.write_all(buffer.as_bytes()).await?;

        match connection_thread(stream, &queue).await {
            Ok(_) => warn!("Disconnected!"),
            Err(e) => warn!("Disconnected: {}", e)
        }
    }
}

async fn connection_thread(stream: TcpStream, queue: &PacketQueue) -> Result<(), Error> {
    let tun = TunBuilder::new()
        .name("gdvpn")
        // TUN = no ethernet headers
        .tap(false)
        .packet_info(false)
        .mtu(1500)
        .address("10.0.0.1".parse()?)
        .destination("10.0.0.2".parse()?)
        .up()
        .try_build()
        .unwrap();

    // IMPORTANT
    // su
    // echo 1 > /proc/sys/net/ipv4/ip_forward
    // iptables -t nat -A POSTROUTING -s 10.0.0.0/8 -o enp4s0 -j MASQUERADE
    // NOTE: enp4s0 = eth device

    let (mut tun_read, mut tun_write) = tokio::io::split(tun);
    let (mut socket_read, mut socket_write) = tokio::io::split(stream);

    // socket -> tun
    let reader = async move {
        loop {
            let mut buf = [0u8; 2];
            socket_read.read_exact(&mut buf).await?;
            let len = i16::from_be_bytes(buf);

            let mut buf = vec![0u8; len as usize];
            let read = socket_read.read_exact(&mut buf).await?;
            if read > 0 {
                // debug!("<< RECV: {}", read);
                tun_write.write_all(&buf[0..read]).await?;
            } else {
                break;
            }
        }
        warn!("Disconnected reader!");
        Ok::<(), Error>(())
    }.fuse();

    // tun -> queue
    // queue -> socket
    let writer = async move {
        loop {
            let mut buf = [0u8; 1500];
            let t1 = tun_read.read(&mut buf).fuse();
            let t2 = queue.recv().fuse();
            pin_mut!(t1, t2);
            select! {
                r = t1 => {
                    let read = r?;
                    if read > 0 {
                        // debug!(">> SEND: {}", read);
                        queue.queue_packet(buf.to_vec()).await;
                    } else {
                        break;
                    }
                },
                r = t2 => {
                    match r {
                        queue::Response::FileIndex(r) => {
                            // Send just file index
                            socket_write.write_all(&(r as u16).to_be_bytes()).await?;
                        },
                    };
                }
            }
        }
        warn!("Disconnected writer");
        Ok::<(), Error>(())
    }.fuse();

    // Wait for disconnect
    pin_mut!(reader, writer);
    select! {
        _ = reader => {
            return Ok(())
        },
        _ = writer => {
            return Ok(())
        }
    }
}