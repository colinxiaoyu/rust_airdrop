# Airdrop - Cross-Platform File Transfer System

A high-performance, secure file transfer system inspired by Apple's AirDrop, built with Rust.

[中文文档](#中文文档) | [English](#english)

---

## English

### Overview

Airdrop is a decentralized peer-to-peer file transfer system that enables seamless file sharing across devices on the same network. Built with Rust for maximum performance and safety, it features automatic device discovery, encrypted transfers, and resumable downloads.

### Features

- 🔍 **Automatic Discovery**: mDNS-based device discovery without manual configuration
- 🔒 **Secure Transfer**: End-to-end encryption with session key management
- ⚡ **High Performance**: QUIC/TCP protocol for fast, reliable transfers
- 🔄 **Resume Support**: Automatic resume for interrupted transfers
- 🎯 **Daemon Architecture**: Background service with CLI/GUI control
- 📦 **Chunked Transfer**: Efficient handling of large files with progress tracking

### Architecture

#### System Components

```
┌───────────────────────┐
│     CLI / GUI         │  User interface layer
│  - Device list        │
│  - Transfer control   │
└─────────────▲─────────┘
              │ IPC / RPC
┌─────────────┴─────────┐
│   airdropd daemon     │  Background service
│                       │
│  ┌─────────────────┐  │
│  │   Discovery     │  │  mDNS/UDP broadcast
│  │   SessionMgr    │  │  Peer management
│  │   Transfer      │  │  QUIC/TCP transfer
│  │   Crypto/Auth   │  │  Encryption & auth
│  └─────────────────┘  │
└───────────────────────┘
```

#### Project Structure

```
airdrop/
├── crates/
│   ├── daemon/          # Background service
│   ├── cli/             # Command-line interface
│   ├── discovery/       # Device discovery (mDNS/UDP)
│   ├── session/         # Session management
│   ├── transfer/        # File transfer (QUIC/TCP)
│   ├── crypto/          # Encryption & authentication
│   └── protocol/        # Message protocol & serialization
└── Cargo.toml           # Workspace configuration
```

### Getting Started

#### Prerequisites

- Rust 2024 edition or later
- Network access on the same subnet

#### Installation

```bash
# Clone the repository
git clone <repository-url>
cd airdrop

# Build the project
cargo build --release

# Install daemon and CLI
cargo install --path crates/daemon
cargo install --path crates/cli
```

#### Usage

```bash
# Start the daemon
airdropd start

# List discovered devices
airdrop list

# Send a file
airdrop send <file> <device-name>

# Receive files (daemon auto-accepts based on config)
airdrop status
```

### Technical Details

#### Event-Driven Architecture

```
Discovery → SessionMgr → Transfer → Crypto → CLI/GUI
   ↓             ↓           ↓         ↓        ↓
PeerFound    NewSession  Progress  Encrypt  Display
PeerLost     EndSession  Complete  Verify   Notify
```

#### Core Technologies

- **Discovery**: mDNS (multicast DNS) for zero-config device discovery
- **Transport**: QUIC for encrypted, multiplexed streams with built-in reliability
- **Fallback**: TCP for environments where UDP is restricted
- **Encryption**: Session-based key exchange with message-level encryption
- **Serialization**: Efficient binary protocol for low overhead

### Roadmap

- [x] Basic daemon architecture
- [x] Device discovery (mDNS)
- [ ] Session management
- [ ] QUIC-based file transfer
- [ ] Encryption layer
- [ ] CLI implementation
- [ ] GUI application
- [ ] Mobile support (iOS/Android)

### Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

### License

[Add your license here]

---

## 中文文档

### 概述

Airdrop 是一个受 Apple AirDrop 启发的高性能、安全的文件传输系统,使用 Rust 构建。它提供了跨平台的点对点文件共享能力,支持自动设备发现、加密传输和断点续传。

### 特性

- 🔍 **自动发现**: 基于 mDNS 的设备自动发现,无需手动配置
- 🔒 **安全传输**: 端到端加密,会话密钥管理
- ⚡ **高性能**: 采用 QUIC/TCP 协议实现快速可靠传输
- 🔄 **断点续传**: 自动恢复中断的传输任务
- 🎯 **守护进程架构**: 后台服务 + CLI/GUI 控制
- 📦 **分块传输**: 高效处理大文件,实时进度跟踪

### 架构设计

#### 整体架构(逻辑视角)

```
┌───────────────────────┐
│      CLI / GUI        │  用户界面层
│  - 命令控制 daemon     │
│  - 显示设备列表        │
│  - 发起/接收文件       │
└─────────────▲─────────┘
              │ IPC / RPC
┌─────────────┴─────────┐
│  airdropd daemon      │  后台常驻服务
│ (systemd / 守护进程)   │
│                       │
│  ┌─────────────────┐  │
│  │   Discovery     │  │  mDNS / UDP 广播
│  │   SessionMgr    │  │  在线 Peer 维护
│  │   Transfer      │  │  QUIC / TCP 传输
│  │   Crypto/Auth   │  │  设备身份 & 加密
│  └─────────────────┘  │
└───────────────────────┘
```

#### Rust Crate 设计(工程化)

```
airdrop/                    # Workspace 根目录
├── crates/
│   ├── daemon/             # 常驻后台服务
│   │   ├── src/main.rs     # 启动、日志、守护循环
│   │   └── config.rs       # 配置加载
│   │
│   ├── cli/                # 命令行控制
│   │   └── src/main.rs
│   │
│   ├── discovery/          # mDNS / UDP 广播 & 发现
│   │   ├── mod.rs
│   │   └── mdns.rs
│   │
│   ├── session/            # Peer 会话管理 & 状态机
│   │   ├── mod.rs
│   │   └── state.rs
│   │
│   ├── transfer/           # QUIC / TCP 文件传输
│   │   ├── mod.rs
│   │   ├── sender.rs
│   │   └── receiver.rs
│   │
│   ├── crypto/             # 身份 & 会话密钥 & 加密
│   │   ├── mod.rs
│   │   └── keys.rs
│   │
│   └── protocol/           # 消息类型 & 序列化
│       ├── mod.rs
│       └── message.rs
└── Cargo.toml              # workspace 配置
```

### 快速开始

#### 环境要求

- Rust 2024 edition 或更高版本
- 同一子网内的网络访问权限

#### 安装

```bash
# 克隆仓库
git clone <repository-url>
cd airdrop

# 构建项目
cargo build --release

# 安装守护进程和 CLI
cargo install --path crates/daemon
cargo install --path crates/cli
```

#### 使用方法

```bash
# 启动守护进程
airdropd start

# 列出发现的设备
airdrop list

# 发送文件
airdrop send <文件路径> <设备名称>

# 接收文件(守护进程根据配置自动接受)
airdrop status
```

### 技术细节

#### 数据 / 事件流

```
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
```

#### 事件驱动设计

- **Discovery → SessionMgr**: `PeerDiscovered` / `PeerLost`
- **SessionMgr → Transfer**: `StartSession` / `StopSession`
- **Transfer → CLI/GUI**: `Progress` / `Done` / `Error`

#### 核心技术

- **设备发现**: mDNS(组播 DNS)实现零配置设备发现
- **传输协议**: QUIC 提供加密、多路复用和内置可靠性
- **降级方案**: TCP 用于 UDP 受限的环境
- **加密方案**: 基于会话的密钥交换和消息级加密
- **序列化**: 高效二进制协议,低开销传输

### 开发路线

- [x] 基础守护进程架构
- [x] 设备发现功能(mDNS)
- [x] 会话管理
- [ ] 基于 QUIC 的文件传输
- [ ] 加密层实现
- [ ] CLI 实现
- [ ] GUI 应用
- [ ] 移动端支持(iOS/Android)
