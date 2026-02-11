import { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener';
import { Menu, RefreshCw, Send, Wifi, WifiOff, FileText, AlertCircle, FolderOpen, ExternalLink } from 'lucide-react';
import { Toaster, toast } from 'sonner';
import { useDaemon } from './hooks/useDaemon';
import { usePeers } from './hooks/usePeers';
import { useFileTransfer } from './hooks/useFileTransfer';
import { useAppStore } from './store';
import { formatFileSize, formatTimestamp } from './lib/tauri';
import clsx from 'clsx';

function App() {
  const { daemonReady, daemonError, deviceInfo } = useDaemon();
  const { peers, loading, error: peersError, refresh } = usePeers();
  const { sendFile } = useFileTransfer();
  const { selectedPeer, selectPeer, transferHistory, sidebarOpen, toggleSidebar } =
    useAppStore();

  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);

  const handleSelectFile = async () => {
    if (!selectedPeer) return;

    try {
      const file = await open({
        multiple: false,
        directory: false,
      });

      if (file) {
        console.log('选择的文件路径:', file);
        setSending(true);
        setSendError(null);

        let filePath = file;

        // Android content:// URI 处理
        if (file.startsWith('content://')) {
          console.warn('检测到 Android content:// URI，尝试解析真实路径');

          // 尝试从 content:// URI 中提取真实路径
          // 格式: content://.../document/raw%3A%2Fstorage%2F...
          const match = file.match(/raw%3A([^&]+)/i);
          if (match) {
            const encodedPath = 'raw:' + match[1];
            const decodedPath = decodeURIComponent(encodedPath);
            filePath = decodedPath.replace('raw:', '');
            console.log('解析出的文件路径:', filePath);
          } else {
            // 尝试其他格式: content://.../document/primary%3A...
            const primaryMatch = file.match(/primary%3A([^&]+)/i);
            if (primaryMatch) {
              const encodedPath = primaryMatch[1];
              const decodedPath = decodeURIComponent(encodedPath);
              filePath = `/storage/emulated/0/${decodedPath}`;
              console.log('解析出的文件路径 (primary):', filePath);
            } else {
              console.error('无法解析 content:// URI:', file);
              setSendError('无法解析文件路径。请尝试：\n1. 使用文件管理器复制文件到 /sdcard/Download/\n2. 然后手动输入路径：/sdcard/Download/文件名');
              setSending(false);
              return;
            }
          }
        }

        await sendFile(selectedPeer.name, filePath);
      }
    } catch (err) {
      console.error('发送文件错误:', err);
      const errorMessage = err instanceof Error ? err.message : '发送失败';
      setSendError(errorMessage);
    } finally {
      setSending(false);
    }
  };

  // 打开文件
  const handleOpenFile = async (filePath: string) => {
    try {
      await openPath(filePath);
    } catch (err) {
      console.error('打开文件失败:', err);
    }
  };

  // 检测是否为移动平台
  const isMobile = () => {
    const platform = navigator.userAgent.toLowerCase();
    return /android|ios|mobile/i.test(platform);
  };

  // 在文件夹中显示（仅桌面平台支持）
  const handleRevealInFolder = async (filePath: string) => {
    if (isMobile()) {
      // Android 无法打开应用私有目录，显示路径提示
      const lastSlashIndex = filePath.lastIndexOf('/');
      const folderPath = lastSlashIndex > 0 ? filePath.substring(0, lastSlashIndex) : filePath;
      toast.info('文件保存位置', {
        description: folderPath,
        duration: 5000,
      });
      return;
    }

    try {
      await revealItemInDir(filePath);
    } catch (err) {
      console.error('打开文件夹失败:', err);
      toast.error('无法打开文件夹');
    }
  };

  console.log('daemonReady',daemonReady)

  // 状态指示器
  const StatusIndicator = () => {
    if (!daemonReady) {
      return (
        <div className="flex items-center gap-2 text-amber-600">
          <div className="w-2 h-2 bg-amber-600 rounded-full animate-pulse" />
          <span className="text-sm">初始化中...</span>
        </div>
      );
    }

    if (daemonError) {
      return (
        <div className="flex items-center gap-2 text-red-600">
          <AlertCircle className="w-4 h-4" />
          <span className="text-sm">{daemonError}</span>
        </div>
      );
    }

    return (
      <div className="flex items-center gap-2 text-green-600">
        <div className="w-2 h-2 bg-green-600 rounded-full" />
        <span className="text-sm">
          在线 {deviceInfo && `· ${deviceInfo.name}`}
        </span>
      </div>
    );
  };

  return (
    <div className="h-screen flex flex-col bg-zinc-50">
      <Toaster position="top-center" richColors />
      {/* Header */}
      <header className="bg-white border-b border-zinc-200 px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <button
              onClick={toggleSidebar}
              className="p-2 hover:bg-zinc-100 rounded-lg transition-colors"
            >
              <Menu className="w-5 h-5" />
            </button>
            <h1 className="text-xl font-bold text-zinc-900">Airdrop</h1>
          </div>
          <StatusIndicator />
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar - 设备列表 */}
        <aside
          className={clsx(
            'bg-white border-r border-zinc-200 transition-all duration-300',
            sidebarOpen ? 'w-80' : 'w-0'
          )}
        >
          <div className="p-4 border-b border-zinc-200">
            <div className="flex items-center justify-between mb-3">
              <h2 className="font-semibold text-zinc-900">
                在线设备 ({peers.length})
              </h2>
              <button
                onClick={refresh}
                disabled={loading}
                className="p-2 hover:bg-zinc-100 rounded-lg transition-colors disabled:opacity-50"
              >
                <RefreshCw className={clsx('w-4 h-4', loading && 'animate-spin')} />
              </button>
            </div>
          </div>

          <div className="overflow-y-auto h-[calc(100vh-180px)]">
            {peersError && (
              <div className="p-4 text-sm text-red-600 flex items-center gap-2">
                <AlertCircle className="w-4 h-4" />
                {peersError}
              </div>
            )}

            {loading && peers.length === 0 && (
              <div className="p-4 text-sm text-zinc-500">加载中...</div>
            )}

            {!loading && peers.length === 0 && (
              <div className="p-4 text-sm text-zinc-500 text-center">
                <WifiOff className="w-8 h-8 mx-auto mb-2 opacity-50" />
                暂无设备在线
              </div>
            )}

            <div className="p-2 space-y-1">
              {peers.map((peer) => (
                <button
                  key={peer.id}
                  onClick={() => selectPeer(peer)}
                  className={clsx(
                    'w-full p-3 rounded-lg text-left transition-colors',
                    selectedPeer?.id === peer.id
                      ? 'bg-blue-50 border border-blue-200'
                      : 'hover:bg-zinc-50 border border-transparent'
                  )}
                >
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 bg-gradient-to-br from-blue-500 to-blue-600 rounded-full flex items-center justify-center text-white font-semibold">
                      {peer.name.charAt(0).toUpperCase()}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-zinc-900 truncate">
                        {peer.name}
                      </div>
                      <div className="text-xs text-zinc-500 truncate">
                        {peer.addr}
                      </div>
                    </div>
                    <Wifi className="w-4 h-4 text-green-600" />
                  </div>
                </button>
              ))}
            </div>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 flex flex-col">
          {/* 发送区域 */}
          <div className="flex-1 flex items-center justify-center p-8">
            {!selectedPeer ? (
              <div className="text-center text-zinc-500">
                <Wifi className="w-16 h-16 mx-auto mb-4 opacity-30" />
                <p className="text-lg mb-2">选择一个设备</p>
                <p className="text-sm">从左侧列表选择要发送文件的设备</p>
              </div>
            ) : (
              <div className="text-center max-w-md">
                <div className="w-20 h-20 bg-gradient-to-br from-blue-500 to-blue-600 rounded-full flex items-center justify-center text-white text-3xl font-semibold mx-auto mb-6">
                  {selectedPeer.name.charAt(0).toUpperCase()}
                </div>
                <h2 className="text-2xl font-bold text-zinc-900 mb-2">
                  {selectedPeer.name}
                </h2>
                <p className="text-sm text-zinc-500 mb-8">{selectedPeer.addr}</p>

                <button
                  onClick={handleSelectFile}
                  disabled={sending || !daemonReady}
                  className="px-8 py-4 bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-3 mx-auto text-lg font-medium shadow-lg shadow-blue-600/30"
                >
                  <Send className="w-5 h-5" />
                  {sending ? '发送中...' : '选择文件发送'}
                </button>

                {sendError && (
                  <div className="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600 flex items-center gap-2">
                    <AlertCircle className="w-4 h-4" />
                    {sendError}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* 传输历史 */}
          {transferHistory.length > 0 && (
            <div className="bg-white border-t border-zinc-200 p-4 max-h-64 overflow-y-auto">
              <h3 className="font-semibold text-zinc-900 mb-3 flex items-center gap-2">
                <FileText className="w-4 h-4" />
                传输历史
              </h3>
              <div className="space-y-2">
                {transferHistory.slice(0, 10).map((transfer) => (
                  <div
                    key={transfer.id}
                    className="flex items-center gap-3 p-3 bg-zinc-50 rounded-lg hover:bg-zinc-100 transition-colors group"
                  >
                    <div
                      className={clsx(
                        'w-8 h-8 rounded-lg flex items-center justify-center',
                        transfer.type === 'sent'
                          ? 'bg-blue-100 text-blue-600'
                          : 'bg-green-100 text-green-600'
                      )}
                    >
                      <Send
                        className={clsx(
                          'w-4 h-4',
                          transfer.type === 'received' && 'rotate-180'
                        )}
                      />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-zinc-900 truncate">
                        {transfer.fileName}
                      </div>
                      <div className="text-xs text-zinc-500">
                        {transfer.type === 'sent' ? '发送到' : '接收自'}{' '}
                        {transfer.peer} · {formatFileSize(transfer.size)}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <div className="text-xs text-zinc-400">
                        {formatTimestamp(transfer.timestamp)}
                      </div>
                      {transfer.status === 'completed' && (
                        <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                          <button
                            onClick={() => handleOpenFile(transfer.file)}
                            className="p-1.5 hover:bg-zinc-200 rounded transition-colors"
                            title="打开文件"
                          >
                            <ExternalLink className="w-3.5 h-3.5 text-zinc-600" />
                          </button>
                          <button
                            onClick={() => handleRevealInFolder(transfer.file)}
                            className="p-1.5 hover:bg-zinc-200 rounded transition-colors"
                            title={isMobile() ? '查看文件位置' : '在文件夹中显示'}
                          >
                            <FolderOpen className="w-3.5 h-3.5 text-zinc-600" />
                          </button>
                        </div>
                      )}
                      {transfer.status === 'failed' && (
                        <AlertCircle className="w-4 h-4 text-red-600" />
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

export default App;
