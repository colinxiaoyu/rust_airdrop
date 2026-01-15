use discovery::Discovery;
use session::{event::SessionEvent, manager::SessionManager};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    println!("🚀 启动 Session Manager 演示...\n");

    // 创建 Discovery 实例
    let mut discovery = Discovery::new("Session演示设备");

    println!("📡 设备信息:");
    println!("  ID: {}", discovery.device_id);
    println!("  名称: {}", discovery.device_name);
    println!();

    // 创建 channel 接收 SessionEvent
    let (event_tx, mut event_rx) = mpsc::channel::<SessionEvent>(100);

    // 创建 SessionManager
    let mut session_manager = SessionManager::new(event_tx);

    println!("🔍 正在监听局域网设备...");
    println!("💡 提示: 在另一个终端运行相同程序来测试会话管理功能");
    println!("⏰ 离线超时: 15秒\n");

    let offline_timeout = Duration::from_secs(15);
    let mut reap_interval = tokio::time::interval(Duration::from_secs(5));
    let mut stats_interval = tokio::time::interval(Duration::from_secs(10));

    let mut peer_count = 0;

    loop {
        tokio::select! {
            // 接收 Discovery 发现的 peer
            Some(peer) = discovery.rx.recv() => {
                peer_count += 1;
                println!("🔍 发现设备 #{}", peer_count);
                println!("   名称: {}", peer.name);
                println!("   地址: {}", peer.addr);
                println!("   ID: {}", peer.id);

                // 更新 SessionManager
                session_manager.on_peer_discovered(peer).await;
            }

            // 接收 SessionEvent
            Some(event) = event_rx.recv() => {
                match event {
                    SessionEvent::PeerOnline(peer) => {
                        println!("✅ 设备上线");
                        println!("   名称: {}", peer.name);
                        println!("   地址: {}\n", peer.addr);
                    }
                    SessionEvent::PeerOffline(peer) => {
                        println!("❌ 设备离线");
                        println!("   名称: {}", peer.name);
                        println!("   地址: {}\n", peer.addr);
                    }
                }
            }

            // 定期清理离线 peer
            _ = reap_interval.tick() => {
                session_manager.reap_offline(offline_timeout).await;
            }

            // 定期打印统计信息
            _ = stats_interval.tick() => {
                println!("📊 统计: 累计发现 {} 个设备\n", peer_count);
            }
        }
    }
}
// cd /Users/colin/Desktop/rust/airdrop
// cargo run --example test_session -p session
