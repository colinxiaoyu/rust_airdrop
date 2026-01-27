import { useEffect, useState } from 'react';
import { tauriApi } from '../lib/tauri';
import { useAppStore } from '../store';

/**
 * Daemon 状态管理 Hook
 * 负责监听 Daemon 就绪和错误事件
 */
export function useDaemon() {
  const { daemonReady, daemonError, setDaemonReady, setDaemonError } = useAppStore();
  const [deviceInfo, setDeviceInfo] = useState<{ name: string; port: number } | null>(null);

  useEffect(() => {
    let mounted = true;

    // 重置状态（每次热重载时）
    setDaemonReady(false);
    setDaemonError(null);

    // 设置超时检测（15秒后如果还没就绪，显示错误提示）
    const timeoutId = setTimeout(() => {
      if (mounted && !daemonReady) {
        setDaemonError('初始化超时，可能端口被占用。请尝试重启应用。');
      }
    }, 15000);

    // 监听 Daemon 事件
    const setupListeners = async () => {
      const unlistenReady = await tauriApi.events.onDaemonReady(() => {
        console.log('Daemon 就绪');
        if (mounted) {
          clearTimeout(timeoutId);
          setDaemonReady(true);
          // 加载设备信息
          loadDeviceInfo();
        }
      });

      const unlistenError = await tauriApi.events.onDaemonError((error) => {
        console.error('Daemon 错误:', error);
        if (mounted) {
          clearTimeout(timeoutId);
          setDaemonError(error);
        }
      });

      return () => {
        unlistenReady();
        unlistenError();
      };
    };

    const cleanup = setupListeners();

    return () => {
      mounted = false;
      clearTimeout(timeoutId);
      cleanup.then((fn) => fn());
    };
  }, [setDaemonReady, setDaemonError]);

  const loadDeviceInfo = async () => {
    try {
      const info = await tauriApi.getDeviceInfo();
      setDeviceInfo(info);
    } catch (err) {
      console.error('加载设备信息失败:', err);
    }
  };

  return { daemonReady, daemonError, deviceInfo };
}
