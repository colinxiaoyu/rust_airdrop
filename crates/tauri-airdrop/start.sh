#!/bin/bash

# Airdrop 启动脚本

echo "🧹 清理旧进程..."
pkill -9 -f "tauri-airdrop" 2>/dev/null
pkill -9 -f "vite.*1420" 2>/dev/null
sleep 1

echo "✨ 启动 Airdrop..."
cd "$(dirname "$0")"
npm run tauri:dev
