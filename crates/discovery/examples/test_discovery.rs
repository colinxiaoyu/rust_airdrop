use discovery::Discovery;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🚀 启动 Discovery 测试...\n");

    // 创建 Discovery 实例
    let mut discovery = Discovery::new("测试设备");

    println!("📡 设备信息:");
    println!("  ID: {}", discovery.device_id);
    println!("  名称: {}", discovery.device_name);
    println!("\n🔍 正在监听局域网设备...");
    println!("💡 提示: 在另一个终端运行相同程序来测试发现功能\n");

    // 监听发现的设备
    let mut peer_count = 0;
    loop {
        tokio::select! {
            Some(peer) = discovery.rx.recv() => {
                peer_count += 1;
                println!("✅ 发现设备 #{}", peer_count);
                println!("   名称: {}", peer.name);
                println!("   地址: {}", peer.addr);
                println!("   ID: {}\n", peer.id);
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                println!("⏰ 30秒检查点 - 已发现 {} 个设备", peer_count);
            }
        }
    }
}
// cd /Users/colin/Desktop/rust/airdrop
// cargo run --example test_discovery -p discovery
