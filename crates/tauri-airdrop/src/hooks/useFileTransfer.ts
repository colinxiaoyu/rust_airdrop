import { useEffect } from 'react';
import { tauriApi } from '../lib/tauri';
import { useAppStore } from '../store';

/**
 * 生成 UUID（兼容 Android）
 */
function generateUUID(): string {
  // 优先使用 crypto.randomUUID
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  // 备用方案：生成简单的 UUID v4
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * 文件传输管理 Hook
 * 负责监听文件接收事件
 */
export function useFileTransfer() {
  const { addTransfer } = useAppStore();

  useEffect(() => {
    const setupListeners = async () => {
      // 监听文件接收事件
      const unlistenReceived = await tauriApi.events.onFileReceived((event) => {
        console.log('收到文件:', event);
        addTransfer({
          id: generateUUID(),
          type: 'received',
          peer: event.from,
          fileName: event.fileName,
          file: event.file,
          size: event.size,
          timestamp: event.timestamp,
          status: 'completed',
        });
      });

      // 监听接收错误事件
      const unlistenError = await tauriApi.events.onReceiveError((event) => {
        console.error('接收错误:', event.error, 'from', event.from);
        addTransfer({
          id: generateUUID(),
          type: 'received',
          peer: event.from,
          fileName: '未知文件',
          file: '',
          size: 0,
          timestamp: new Date().toISOString(),
          status: 'failed',
          error: event.error,
        });
      });

      return () => {
        unlistenReceived();
        unlistenError();
      };
    };

    const cleanup = setupListeners();

    return () => {
      cleanup.then((fn) => fn());
    };
  }, [addTransfer]);

  /**
   * 发送文件
   */
  const sendFile = async (peerName: string, filePath: string) => {
    try {
      // 先获取文件信息
      const fileInfo = await tauriApi.getFileInfo(filePath);

      // 添加到传输历史（使用真实的文件大小）
      addTransfer({
        id: generateUUID(),
        type: 'sent',
        peer: peerName,
        fileName: fileInfo.name,
        file: filePath,
        size: fileInfo.size,
        timestamp: new Date().toISOString(),
        status: 'completed',
      });

      // 发送文件
      await tauriApi.sendFile(peerName, filePath);
    } catch (err) {
      throw err;
    }
  };

  return { sendFile };
}
