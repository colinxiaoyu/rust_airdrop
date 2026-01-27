import { useEffect, useState } from 'react';
import { tauriApi } from '../lib/tauri';
import { useAppStore } from '../store';

/**
 * 设备列表管理 Hook
 * 负责加载设备列表、监听设备上下线事件
 */
export function usePeers() {
  const { peers, addPeer, removePeer, setPeers } = useAppStore();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    // 加载初始设备列表
    const loadPeers = async () => {
      try {
        const data = await tauriApi.listPeers();
        if (mounted) {
          setPeers(data);
          setError(null);
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : '加载设备失败');
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    loadPeers();

    // 监听设备上线事件
    const setupListeners = async () => {
      const unlistenOnline = await tauriApi.events.onPeerOnline((peer) => {
        console.log('设备上线:', peer);
        addPeer(peer);
      });

      const unlistenOffline = await tauriApi.events.onPeerOffline((peer) => {
        console.log('设备下线:', peer);
        removePeer(peer.id);
      });

      return () => {
        unlistenOnline();
        unlistenOffline();
      };
    };

    const cleanup = setupListeners();

    return () => {
      mounted = false;
      cleanup.then((fn) => fn());
    };
  }, [addPeer, removePeer, setPeers]);

  const refresh = async () => {
    setLoading(true);
    try {
      const data = await tauriApi.listPeers();
      setPeers(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : '刷新失败');
    } finally {
      setLoading(false);
    }
  };

  return { peers, loading, error, refresh };
}
