import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

// ============ 类型定义 ============

export interface Peer {
  id: string;
  name: string;
  addr: string;
}

export interface DeviceInfo {
  name: string;
  port: number;
}

export interface FileReceivedEvent {
  from: string;
  fileName: string;
  file: string;
  size: number;
  timestamp: string;
}

export interface ReceiveErrorEvent {
  from: string;
  error: string;
}

// ============ API 封装 ============

/**
 * Tauri 后端 API 封装
 * 提供类型安全的 Rust 后端调用接口
 */
export const tauriApi = {
  // ---- 设备管理 ----

  /**
   * 获取在线设备列表
   */
  listPeers: async (): Promise<Peer[]> => {
    return invoke<Peer[]>('list_peers');
  },

  /**
   * 获取本设备信息
   */
  getDeviceInfo: async (): Promise<DeviceInfo> => {
    return invoke<DeviceInfo>('get_device_info');
  },

  // ---- 文件传输 ----

  /**
   * 发送文件到指定设备
   * @param peerName 目标设备名称
   * @param filePath 文件路径
   */
  sendFile: async (peerName: string, filePath: string): Promise<void> => {
    return invoke<void>('send_file', { peerName, filePath });
  },

  // ---- 文件选择 ----

  /**
   * 获取下载目录
   */
  getDownloadDir: async (): Promise<string> => {
    return invoke<string>('get_download_dir');
  },

  /**
   * 检查 Daemon 是否已就绪
   */
  checkDaemonReady: async (): Promise<boolean> => {
    return invoke<boolean>('check_daemon_ready');
  },

  // ---- 事件监听 ----

  events: {
    /**
     * 监听设备上线事件
     */
    onPeerOnline: (callback: (peer: Peer) => void): Promise<UnlistenFn> => {
      return listen<Peer>('peer-online', (event) => callback(event.payload));
    },

    /**
     * 监听设备下线事件
     */
    onPeerOffline: (callback: (peer: Peer) => void): Promise<UnlistenFn> => {
      return listen<Peer>('peer-offline', (event) => callback(event.payload));
    },

    /**
     * 监听文件接收事件
     */
    onFileReceived: (callback: (event: FileReceivedEvent) => void): Promise<UnlistenFn> => {
      return listen<FileReceivedEvent>('file-received', (event) => callback(event.payload));
    },

    /**
     * 监听接收错误事件
     */
    onReceiveError: (callback: (event: ReceiveErrorEvent) => void): Promise<UnlistenFn> => {
      return listen<ReceiveErrorEvent>('receive-error', (event) => callback(event.payload));
    },

    /**
     * 监听 Daemon 就绪事件
     */
    onDaemonReady: (callback: () => void): Promise<UnlistenFn> => {
      return listen('daemon-ready', () => callback());
    },

    /**
     * 监听 Daemon 错误事件
     */
    onDaemonError: (callback: (error: string) => void): Promise<UnlistenFn> => {
      return listen<string>('daemon-error', (event) => callback(event.payload));
    },
  },
};

// ============ 工具函数 ============

/**
 * 格式化文件大小
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

/**
 * 从文件路径提取文件名
 */
export function getFileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

/**
 * 格式化时间戳
 */
export function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  // 小于 1 分钟
  if (diff < 60000) {
    return '刚刚';
  }

  // 小于 1 小时
  if (diff < 3600000) {
    const minutes = Math.floor(diff / 60000);
    return `${minutes} 分钟前`;
  }

  // 小于 1 天
  if (diff < 86400000) {
    const hours = Math.floor(diff / 3600000);
    return `${hours} 小时前`;
  }

  // 大于 1 天，显示日期
  return date.toLocaleString('zh-CN', {
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}
