use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::{net::UdpSocket, sync::mpsc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: Uuid,
    pub name: String,
    pub addr: SocketAddr,
}

pub struct Discovery {
    pub device_id: Uuid,
    pub device_name: String,
    pub rx: mpsc::Receiver<Peer>,
}

impl Discovery {
    pub fn new(device_name: &str) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let device_id = Uuid::new_v4();

        let device_name = device_name.to_string();

        tokio::spawn(Self::broadcast_task(device_name.clone()));

        tokio::spawn(Self::listen_task(tx.clone()));

        Discovery {
            device_id,
            device_name,
            rx,
        }
    }

    /// 广播自己的存在
    async fn broadcast_task(device_name: String) {
        let muticast_addr = "224.0.0.251:5353";
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("UdpSocket unstart");

        let msg = format!("DISCOVERY:{}\n", device_name);

        loop {
            let _ = socket.send_to(msg.as_bytes(), muticast_addr).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// 监听局域网内其他设备
    async fn listen_task(tx: mpsc::Sender<Peer>) {
        // 1️⃣ 创建 socket2 套接字
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
            .expect("Failed to create socket");

        // 2️⃣ 设置端口复用
        socket
            .set_reuse_address(true)
            .expect("set_reuse_address failed");
        #[cfg(unix)]
        socket.set_reuse_port(true).expect("set_reuse_port failed"); // Unix 平台可选

        // 3️⃣ 绑定本地端口
        let addr: SocketAddr = "0.0.0.0:5353".parse().unwrap();
        socket.bind(&addr.into()).expect("bind failed");

        // 4️⃣ 加入多播组
        socket
            .join_multicast_v4(&Ipv4Addr::new(224, 0, 0, 251), &Ipv4Addr::UNSPECIFIED)
            .expect("join_multicast_v4 failed");

        // 5️⃣ 设置非阻塞模式 (Tokio 需要)
        socket
            .set_nonblocking(true)
            .expect("set_nonblocking failed");

        let socket = UdpSocket::from_std(socket.into()).expect("from_std failed");

        // 7️⃣ 循环接收
        let mut buf = [0u8; 1025];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    if let Ok(msg) = str::from_utf8(&buf[..len]) {
                        if msg.starts_with("DISCOVERY:") {
                            let name = msg.trim_start_matches("DISCOVERY:").trim().to_string();
                            let peer = Peer {
                                id: Uuid::new_v4(),
                                name,
                                addr,
                            };
                            let _ = tx.send(peer).await;
                        }
                    }
                }
                Err(e) => eprintln!("Discovery recv error: {:?}", e),
            }
        }
    }
}
