import { create } from 'zustand';
import { Peer, FileReceivedEvent } from '../lib/tauri';

// ============ 类型定义 ============

export interface TransferHistory {
  id: string;
  type: 'sent' | 'received';
  peer: string;
  fileName: string;
  file: string;
  size: number;
  timestamp: string;
  status: 'completed' | 'failed';
  error?: string;
}

export interface AppState {
  // Daemon 状态
  daemonReady: boolean;
  daemonError: string | null;

  // 设备列表
  peers: Peer[];
  selectedPeer: Peer | null;

  // 传输历史
  transferHistory: TransferHistory[];

  // UI 状态
  sidebarOpen: boolean;

  // Actions
  setDaemonReady: (ready: boolean) => void;
  setDaemonError: (error: string | null) => void;
  setPeers: (peers: Peer[]) => void;
  addPeer: (peer: Peer) => void;
  removePeer: (peerId: string) => void;
  selectPeer: (peer: Peer | null) => void;
  addTransfer: (transfer: TransferHistory) => void;
  toggleSidebar: () => void;
}

// ============ Store ============

export const useAppStore = create<AppState>((set) => ({
  // 初始状态
  daemonReady: false,
  daemonError: null,
  peers: [],
  selectedPeer: null,
  transferHistory: [],
  sidebarOpen: true,

  // Daemon 状态
  setDaemonReady: (ready) =>
    set({ daemonReady: ready, daemonError: ready ? null : undefined }),
  setDaemonError: (error) => set({ daemonError: error, daemonReady: false }),

  // 设备管理
  setPeers: (peers) => set({ peers }),
  addPeer: (peer) =>
    set((state) => {
      const exists = state.peers.find((p) => p.id === peer.id);
      return exists ? state : { peers: [...state.peers, peer] };
    }),
  removePeer: (peerId) =>
    set((state) => ({
      peers: state.peers.filter((p) => p.id !== peerId),
      selectedPeer: state.selectedPeer?.id === peerId ? null : state.selectedPeer,
    })),
  selectPeer: (peer) => set({ selectedPeer: peer }),

  // 传输历史
  addTransfer: (transfer) =>
    set((state) => ({
      transferHistory: [transfer, ...state.transferHistory].slice(0, 100), // 最多保留 100 条
    })),

  // UI 状态
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
}));
