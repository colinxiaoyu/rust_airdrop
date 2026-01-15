一、整体架构图（逻辑视角）
┌───────────────────────┐
│        CLI / GUI       │
│  - 命令控制 daemon     │
│  - 显示设备列表        │
│  - 发起/接收文件       │
└─────────────▲─────────┘
              │ IPC / RPC
              │
┌─────────────┴─────────┐
│     airdropd daemon    │
│ (后台常驻服务 / systemd) │
│                        │
│  ┌───────────────┐     │
│  │ Discovery     │◄──┐ │
│  │ - mDNS / UDP  │   │ │
│  │ - 广播服务    │   │ │
│  │ - 发现新设备  │   │ │
│  └───────────────┘   │ │
│                      │ │
│  ┌───────────────┐   │ │
│  │ SessionMgr    │   │ │
│  │ - 在线 Peer 维护 │ │ │
│  │ - 握手 / 验证 │   │ │
│  │ - 会话状态机 │   │ │
│  └───────────────┘   │ │
│                      │ │
│  ┌───────────────┐   │ │
│  │ Transfer      │   │ │
│  │ - QUIC / TCP  │   │ │
│  │ - Chunk 文件  │   │ │
│  │ - Resume / 校验 │ │ │
│  └───────────────┘   │ │
│                      │ │
│  ┌───────────────┐   │ │
│  │ Crypto / Auth │   │ │
│  │ - 设备身份    │   │ │
│  │ - 会话密钥    │   │ │
│  │ - 消息加密    │   │ │
│  └───────────────┘   │ │
└───────────────────────┘
二、Rust crate 设计（工程化）
airdrop/                       # Workspace 根目录
├── crates/
│   ├── daemon/                # 常驻后台服务
│   │   ├── src/main.rs        # 启动、日志、守护循环
│   │   └── config.rs          # 配置加载
│   │
│   ├── cli/                   # 命令行控制
│   │   └── src/main.rs
│   │
│   ├── discovery/             # mDNS / UDP 广播 & 发现
│   │   ├── mod.rs
│   │   └── mdns.rs
│   │
│   ├── session/               # Peer 会话管理 & 状态机
│   │   ├── mod.rs
│   │   └── state.rs
│   │
│   ├── transfer/              # QUIC / TCP 文件传输
│   │   ├── mod.rs
│   │   ├── sender.rs
│   │   └── receiver.rs
│   │
│   ├── crypto/                # 身份 & 会话密钥 & 加密
│   │   ├── mod.rs
│   │   └── keys.rs
│   │
│   └── protocol/              # 消息类型 & 序列化
│       ├── mod.rs
│       └── message.rs
└── Cargo.toml                 # workspace
三、数据 / 事件流
Discovery (mDNS)
       │ 广播 + 发现事件
       ▼
SessionMgr
       │ 建立/维护会话
       ▼
Transfer Core (QUIC/TCP)
       │ 文件分块、传输、断点续传
       ▼
Crypto
       │ 消息加密/解密、身份校验
       ▼
CLI / GUI
       │ 展示设备列表、进度条、操作
事件驱动设计：

Discovery → SessionMgr：PeerDiscovered / PeerLost

SessionMgr → Transfer：StartSession / StopSession

Transfer → CLI/GUI：Progress / Done / Error