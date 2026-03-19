import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Loader2,
  FolderOpen,
  FileCode,
  Trash2,
  AlertTriangle,
  X,
} from 'lucide-react';

interface InstallResult {
  success: boolean;
  message: string;
  error?: string;
}

interface SettingsProps {
  onEnvironmentChange?: () => void;
}

export function Settings({ onEnvironmentChange }: SettingsProps) {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const [uninstalling, setUninstalling] = useState(false);
  const [uninstallResult, setUninstallResult] = useState<InstallResult | null>(null);

  const openDir = async (subpath?: string) => {
    try {
      const info = await invoke<{ config_dir: string }>('get_system_info');
      const path = subpath ? `${info.config_dir}/${subpath}` : info.config_dir;
      await invoke('open_dir', { path });
    } catch (e) {
      console.error('打开目录失败:', e);
    }
  };

  const handleUninstall = async () => {
    setUninstalling(true);
    setUninstallResult(null);
    try {
      const result = await invoke<InstallResult>('uninstall_openclaw');
      setUninstallResult(result);
      if (result.success) {
        // 通知环境状态变化，触发重新检查
        onEnvironmentChange?.();
        // 卸载成功后关闭确认框
        setTimeout(() => {
          setShowUninstallConfirm(false);
        }, 2000);
      }
    } catch (e) {
      setUninstallResult({
        success: false,
        message: '卸载过程中发生错误',
        error: String(e),
      });
    } finally {
      setUninstalling(false);
    }
  };

  return (
    <div className="h-full overflow-y-auto scroll-container">
      <div className="max-w-xl space-y-4">
        {/* 高级设置 */}
        <div className="card p-5">
          <div className="flex items-center gap-3 mb-5">
            <div className="w-8 h-8 rounded-lg bg-purple-500/10 flex items-center justify-center">
              <FileCode size={16} className="text-purple-500" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-gray-900 dark:text-white">高级设置</h3>
              <p className="text-xs text-gray-400 dark:text-gray-500">配置文件和目录</p>
            </div>
          </div>

          <div className="space-y-2">
            <button
              onClick={() => openDir()}
              className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg
                bg-gray-50 dark:bg-white/[0.03] hover:bg-gray-100 dark:hover:bg-white/[0.06]
                transition-colors text-left"
            >
              <FolderOpen size={15} className="text-gray-400 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm text-gray-900 dark:text-white">打开安装目录</p>
                <p className="text-xs text-gray-400 font-mono truncate">~/.openclaw/</p>
              </div>
            </button>
            <button
              onClick={() => openDir('logs')}
              className="w-full flex items-center gap-3 px-3 py-2.5 rounded-lg
                bg-gray-50 dark:bg-white/[0.03] hover:bg-gray-100 dark:hover:bg-white/[0.06]
                transition-colors text-left"
            >
              <FolderOpen size={15} className="text-gray-400 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm text-gray-900 dark:text-white">打开日志目录</p>
                <p className="text-xs text-gray-400 font-mono truncate">~/.openclaw/logs/</p>
              </div>
            </button>
          </div>
        </div>

        {/* 危险操作 */}
        <div className="card p-5 border-red-100 dark:border-red-500/10">
          <div className="flex items-center gap-3 mb-5">
            <div className="w-8 h-8 rounded-lg bg-red-500/10 flex items-center justify-center">
              <AlertTriangle size={16} className="text-red-500" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-gray-900 dark:text-white">危险操作</h3>
              <p className="text-xs text-gray-400 dark:text-gray-500">以下操作不可撤销</p>
            </div>
          </div>

          <div className="space-y-3">
            <button
              onClick={() => setShowUninstallConfirm(true)}
              className="w-full flex items-center gap-3 p-4 bg-red-50 dark:bg-red-950/30 rounded-lg hover:bg-red-100 dark:hover:bg-red-900/40 transition-colors text-left border border-red-200 dark:border-red-900/30"
            >
              <Trash2 size={18} className="text-red-500 dark:text-red-400" />
              <div className="flex-1">
                <p className="text-sm text-red-600 dark:text-red-300">卸载 OpenClaw</p>
                <p className="text-xs text-red-500/70 dark:text-red-400/70">从系统中移除 OpenClaw CLI 工具</p>
              </div>
            </button>
          </div>
        </div>

        {/* 卸载确认对话框 */}
        {showUninstallConfirm && (
          <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
            <div className="bg-white dark:bg-dark-700 rounded-2xl p-6 border border-gray-200 dark:border-dark-500 max-w-md w-full mx-4 shadow-2xl">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 rounded-xl bg-red-500/20 flex items-center justify-center">
                    <AlertTriangle size={20} className="text-red-500 dark:text-red-400" />
                  </div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-white">确认卸载</h3>
                </div>
                <button
                  onClick={() => {
                    setShowUninstallConfirm(false);
                    setUninstallResult(null);
                  }}
                  className="text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors"
                >
                  <X size={20} />
                </button>
              </div>

              {!uninstallResult ? (
                <>
                  <p className="text-gray-600 dark:text-gray-300 mb-4">
                    确定要卸载 OpenClaw 吗？此操作将：
                  </p>
                  <ul className="text-sm text-gray-500 dark:text-gray-400 mb-6 space-y-2">
                    <li className="flex items-center gap-2">
                      <span className="w-1.5 h-1.5 bg-red-400 rounded-full"></span>
                      停止正在运行的服务
                    </li>
                    <li className="flex items-center gap-2">
                      <span className="w-1.5 h-1.5 bg-red-400 rounded-full"></span>
                      移除 OpenClaw CLI 工具
                    </li>
                    <li className="flex items-center gap-2">
                      <span className="w-1.5 h-1.5 bg-yellow-400 rounded-full"></span>
                      配置文件将被保留在 ~/.openclaw
                    </li>
                  </ul>

          <div className="flex gap-3">
                    <button
                      onClick={() => setShowUninstallConfirm(false)}
                      className="flex-1 px-4 py-2.5 bg-gray-100 dark:bg-dark-600 hover:bg-gray-200 dark:hover:bg-dark-500 text-gray-900 dark:text-white rounded-lg transition-colors"
                    >
                      取消
                    </button>
                    <button
                      onClick={handleUninstall}
                      disabled={uninstalling}
                      className="flex-1 px-4 py-2.5 bg-red-600 hover:bg-red-500 text-white rounded-lg transition-colors flex items-center justify-center gap-2 disabled:opacity-50"
                    >
                      {uninstalling ? (
                        <>
                          <Loader2 size={16} className="animate-spin" />
                          卸载中...
                        </>
                      ) : (
                        <>
                          <Trash2 size={16} />
                          确认卸载
                        </>
                      )}
                    </button>
                  </div>
                </>
              ) : (
                <div className={`p-4 rounded-lg ${uninstallResult.success ? 'bg-green-900/30 border border-green-800' : 'bg-red-900/30 border border-red-800'}`}>
                  <p className={`text-sm ${uninstallResult.success ? 'text-green-300' : 'text-red-300'}`}>
                    {uninstallResult.message}
                  </p>
                  {uninstallResult.error && (
                    <p className="text-xs text-red-400 mt-2 font-mono">
                      {uninstallResult.error}
                    </p>
                  )}
                  {uninstallResult.success && (
                    <p className="text-xs text-gray-400 mt-3">
                      对话框将自动关闭...
                    </p>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

      </div>
    </div>
  );
}
