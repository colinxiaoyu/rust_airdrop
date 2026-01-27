import { useEffect } from 'react';
import { tauriApi, getFileName } from '../lib/tauri';
import { useAppStore } from '../store';

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
          id: crypto.randomUUID(),
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
        console.error('接收错误:', event);
        addTransfer({
          id: crypto.randomUUID(),
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
    // 添加到传输历史
    addTransfer({
      id: crypto.randomUUID(),
      type: 'sent',
      peer: peerName,
      fileName: getFileName(filePath),
      file: filePath,
      size: 0,
      timestamp: new Date().toISOString(),
      status: 'completed',
    });

    try {
      await tauriApi.sendFile(peerName, filePath);
    } catch (err) {
      throw err;
    }
  };

  return { sendFile };
}
