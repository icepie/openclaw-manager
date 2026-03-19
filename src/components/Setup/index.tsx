import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import {
  CheckCircle2,
  Loader2,
  Download,
  ArrowRight,
  RefreshCw,
  Package,
  Pencil,
  X,
  Check,
  FolderOpen,
} from 'lucide-react';
import { setupLogger } from '../../lib/logger';

interface EnvironmentStatus {
  git_installed: boolean;
  git_version: string | null;
  node_installed: boolean;
  node_version: string | null;
  node_version_ok: boolean;
  openclaw_installed: boolean;
  openclaw_version: string | null;
  config_dir_exists: boolean;
  ready: boolean;
  os: string;
}

interface InstallResult {
  success: boolean;
  message: string;
  error: string | null;
}

interface InstallProgress {
  step: string;
  progress: number;
  message: string;
  error: string | null;
}

interface DownloadProgress {
  downloaded: number;
  total: number | null;
  percent: number | null;
}

interface SetupProps {
  onComplete: () => void;
  embedded?: boolean;
}

export function Setup({ onComplete, embedded = false }: SetupProps) {
  const [, setEnvStatus] = useState<EnvironmentStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [step, setStep] = useState<'check' | 'install' | 'complete'>('check');
  const [bundleUrl, setBundleUrl] = useState('');
  const [editingUrl, setEditingUrl] = useState(false);
  const [editUrl, setEditUrl] = useState('');
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [installProgress, setInstallProgress] = useState<InstallProgress | null>(null);
  const [installDir, setInstallDir] = useState<string | null>(null);
  const [localBundlePath, setLocalBundlePath] = useState<string | null>(null);
  const [installStatus, setInstallStatus] = useState('');

  const checkEnvironment = async () => {
    setupLogger.info('检查系统环境...');
    setChecking(true);
    setError(null);
    try {
      const status = await invoke<EnvironmentStatus>('check_environment');
      setEnvStatus(status);
      if (status.openclaw_installed) {
        setStep('complete');
        setTimeout(() => onComplete(), 1500);
      } else {
        setStep('install');
      }
    } catch (e) {
      setError(`检查环境失败: ${e}`);
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    invoke<string>('get_bundle_download_url').then(setBundleUrl);
    checkEnvironment();
  }, []);

  const handleInstallOpenclaw = async () => {
    setupLogger.action('安装 OpenClaw');
    setInstalling(true);
    setError(null);
    setProgress(null);
    setInstallProgress(null);
    setInstallStatus('正在准备安装...');

    const unlistenDownload = await listen<DownloadProgress>('bundle-download-progress', (e) => {
      setProgress(e.payload);
      setInstallStatus('正在下载 bundle...');
    });

    const unlistenInstall = await listen<InstallProgress>('install-progress', (e) => {
      setInstallProgress(e.payload);
      setInstallStatus(e.payload.message);
    });

    try {
      setInstallStatus('正在检查本地 bundle...');
      const result = await invoke<InstallResult>('install_openclaw', {
        bundleUrl: localBundlePath ? null : (bundleUrl || null),
        localBundlePath: localBundlePath || null,
        installDir: installDir || null,
      });

      if (result.success) {
        setInstallStatus('正在初始化配置...');
        await invoke<InstallResult>('init_openclaw_config');
        await checkEnvironment();
      } else {
        setError(result.error || result.message || '安装失败，请检查 URL 后重试');
      }
    } catch (e) {
      setError(`安装失败: ${e}`);
    } finally {
      unlistenDownload();
      unlistenInstall();
      setInstalling(false);
      setProgress(null);
      setInstallProgress(null);
      setInstallStatus('');
    }
  };

  const handlePickDir = async () => {
    const selected = await open({ directory: true, multiple: false, title: '选择安装目录' });
    if (typeof selected === 'string') setInstallDir(selected);
  };

  const handlePickLocalBundle = async () => {
    const selected = await open({
      multiple: false,
      title: '选择本地离线包',
      filters: [{ name: 'Bundle', extensions: ['tar.gz', 'tgz', 'zip', 'gz'] }],
    });
    if (typeof selected === 'string') setLocalBundlePath(selected);
  };

  const startEditUrl = () => {
    setEditUrl(bundleUrl);
    setEditingUrl(true);
  };

  const confirmEditUrl = () => {
    if (editUrl.trim()) setBundleUrl(editUrl.trim());
    setEditingUrl(false);
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  };

  const renderContent = () => {
    return (
      <AnimatePresence mode="wait">
        {/* 检查中 */}
        {checking && (
          <motion.div
            key="checking"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="text-center py-6"
          >
            <Loader2 className="w-10 h-10 text-claw-500 animate-spin mx-auto mb-3" />
            <p className="text-gray-500 dark:text-gray-400">正在检测系统环境...</p>
          </motion.div>
        )}

        {/* 安装步骤 */}
        {!checking && step === 'install' && (
          <motion.div
            key="install"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="space-y-4"
          >
            {/* OpenClaw 状态行 */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-red-500/20 text-red-500 dark:text-red-400">
                  <Package className="w-5 h-5" />
                </div>
                <div>
                  <p className="text-gray-900 dark:text-white font-medium">OpenClaw</p>
                  <p className="text-sm text-gray-500 dark:text-gray-400">未安装</p>
                </div>
              </div>

              <button
                onClick={handleInstallOpenclaw}
                disabled={installing}
                className="btn-primary text-sm px-4 py-2 flex items-center gap-2"
              >
                {installing ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {installStatus || '安装中...'}
                  </>
                ) : (
                  <>
                    <Download className="w-4 h-4" />
                    安装
                  </>
                )}
              </button>
              {installing && (
                <button
                  onClick={() => invoke('cancel_install')}
                  className="text-sm px-3 py-2 rounded-lg border border-red-300 dark:border-red-500/30 text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 transition-colors"
                >
                  取消
                </button>
              )}
            </div>

            {/* 安装状态文字 */}
            {installing && installStatus && !progress && !installProgress && (
              <p className="text-xs text-gray-500 dark:text-gray-400 text-center">{installStatus}</p>
            )}

            {/* 安装进度（各阶段） */}
            {installing && installProgress && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                className="space-y-1.5"
              >
                <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
                  <span>{installProgress.message}</span>
                  <span>{installProgress.progress}%</span>
                </div>
                <div className="w-full h-1.5 bg-gray-200 dark:bg-white/10 rounded-full overflow-hidden">
                  <motion.div
                    className="h-full bg-claw-500 rounded-full"
                    initial={{ width: 0 }}
                    animate={{ width: `${installProgress.progress}%` }}
                    transition={{ ease: 'linear', duration: 0.3 }}
                  />
                </div>
              </motion.div>
            )}

            {/* 下载进度 */}
            {installing && progress && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                className="space-y-1.5"
              >
                <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
                  <span>{formatBytes(progress.downloaded)}{progress.total ? ` / ${formatBytes(progress.total)}` : ''}</span>
                  <span>{progress.percent != null ? `${progress.percent.toFixed(1)}%` : ''}</span>
                </div>
                <div className="w-full h-1.5 bg-gray-200 dark:bg-white/10 rounded-full overflow-hidden">
                  <motion.div
                    className="h-full bg-claw-500 rounded-full"
                    initial={{ width: 0 }}
                    animate={{ width: `${progress.percent ?? 0}%` }}
                    transition={{ ease: 'linear', duration: 0.2 }}
                  />
                </div>
              </motion.div>
            )}

            {/* Bundle URL / 本地离线包 */}
            <div className="pt-1 space-y-2">
              <div className="flex items-center justify-between">
                <p className="text-xs text-gray-400 dark:text-gray-500">Bundle 来源</p>
                {localBundlePath && (
                  <button onClick={() => setLocalBundlePath(null)} disabled={installing}
                    className="text-xs text-gray-400 hover:text-red-400 transition-colors flex items-center gap-1">
                    <X className="w-3 h-3" />使用远程 URL
                  </button>
                )}
              </div>

              {localBundlePath ? (
                <div className="flex items-center gap-2">
                  <p className="flex-1 text-xs text-green-600 dark:text-green-400 truncate font-mono bg-green-500/10 border border-green-500/20 rounded-lg px-2.5 py-1.5">
                    {localBundlePath}
                  </p>
                  <button onClick={handlePickLocalBundle} disabled={installing} className="icon-btn" title="重新选择">
                    <FolderOpen className="w-4 h-4" />
                  </button>
                </div>
              ) : (
                <>
                  {editingUrl ? (
                    <div className="flex items-center gap-2">
                      <input
                        className="input-base flex-1 text-xs py-1.5"
                        value={editUrl}
                        onChange={(e) => setEditUrl(e.target.value)}
                        onKeyDown={(e) => e.key === 'Enter' && confirmEditUrl()}
                        autoFocus
                      />
                      <button onClick={confirmEditUrl} className="icon-btn text-green-500">
                        <Check className="w-4 h-4" />
                      </button>
                      <button onClick={() => setEditingUrl(false)} className="icon-btn">
                        <X className="w-4 h-4" />
                      </button>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2 group">
                      <p className="flex-1 text-xs text-gray-500 dark:text-gray-400 truncate font-mono bg-gray-100 dark:bg-white/[0.04] border border-gray-200 dark:border-white/[0.06] rounded-lg px-2.5 py-1.5">
                        {bundleUrl}
                      </p>
                      <button onClick={startEditUrl} disabled={installing} className="icon-btn opacity-0 group-hover:opacity-100 transition-opacity" title="编辑 URL">
                        <Pencil className="w-3.5 h-3.5" />
                      </button>
                      <button onClick={handlePickLocalBundle} disabled={installing} className="icon-btn opacity-0 group-hover:opacity-100 transition-opacity" title="选择本地文件">
                        <FolderOpen className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  )}
                </>
              )}
            </div>

            {/* 安装目录 */}
            <div className="pt-1">
              <p className="text-xs text-gray-400 dark:text-gray-500 mb-1.5">安装目录（留空使用默认 ~/.openclaw）</p>
              <div className="flex items-center gap-2">
                <p className="flex-1 text-xs text-gray-500 dark:text-gray-400 truncate font-mono bg-gray-100 dark:bg-white/[0.04] border border-gray-200 dark:border-white/[0.06] rounded-lg px-2.5 py-1.5">
                  {installDir || '~/.openclaw（默认）'}
                </p>
                <button onClick={handlePickDir} disabled={installing} className="icon-btn" title="选择目录">
                  <FolderOpen className="w-4 h-4" />
                </button>
                {installDir && (
                  <button onClick={() => setInstallDir(null)} disabled={installing} className="icon-btn">
                    <X className="w-4 h-4" />
                  </button>
                )}
              </div>
            </div>

            {/* 错误信息 */}
            {error && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="p-3 bg-yellow-500/10 border border-yellow-500/30 rounded-lg"
              >
                <p className="text-yellow-600 dark:text-yellow-400 text-sm">{error}</p>
              </motion.div>
            )}

            {/* 重新检查 */}
            <div className="flex gap-3 pt-4 border-t border-gray-200 dark:border-white/[0.08]">
              <button
                onClick={checkEnvironment}
                disabled={checking || installing}
                className="flex-1 btn-secondary py-2.5 flex items-center justify-center gap-2"
              >
                <RefreshCw className={`w-4 h-4 ${checking ? 'animate-spin' : ''}`} />
                重新检查
              </button>
            </div>
          </motion.div>
        )}

        {/* 完成状态 */}
        {!checking && step === 'complete' && (
          <motion.div
            key="complete"
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            className="text-center py-6"
          >
            <motion.div
              initial={{ scale: 0 }}
              animate={{ scale: 1 }}
              transition={{ type: 'spring', damping: 10, delay: 0.1 }}
            >
              <CheckCircle2 className="w-12 h-12 text-green-500 dark:text-green-400 mx-auto mb-3" />
            </motion.div>
            <h3 className="text-lg font-bold text-gray-900 dark:text-white mb-1">环境就绪！</h3>
            <p className="text-gray-500 dark:text-gray-400 text-sm">OpenClaw 已正确安装</p>
            <button
              onClick={onComplete}
              className="mt-4 btn-primary py-2.5 px-6 flex items-center justify-center gap-2 mx-auto"
            >
              开始使用
              <ArrowRight className="w-4 h-4" />
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    );
  };

  if (embedded) {
    return (
      <div className="bg-yellow-500/10 border border-yellow-500/30 rounded-2xl p-6">
        <div className="flex items-start gap-4 mb-4">
          <div className="flex-shrink-0 w-12 h-12 rounded-xl bg-gradient-to-br from-yellow-500 to-orange-500 flex items-center justify-center">
            <span className="text-2xl">⚠️</span>
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-1">环境配置</h2>
            <p className="text-gray-500 dark:text-gray-400 text-sm">检测到 OpenClaw 未安装，请完成安装</p>
          </div>
        </div>
        {renderContent()}
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-[#0d0d0f] flex items-center justify-center p-8">
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute -top-40 -right-40 w-80 h-80 bg-claw-500/10 rounded-full blur-3xl" />
        <div className="absolute -bottom-40 -left-40 w-80 h-80 bg-purple-500/10 rounded-full blur-3xl" />
      </div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="relative z-10 w-full max-w-md"
      >
        <div className="text-center mb-8">
          <motion.div
            initial={{ scale: 0.8 }}
            animate={{ scale: 1 }}
            transition={{ type: 'spring', damping: 15 }}
            className="inline-flex items-center justify-center w-20 h-20 rounded-2xl bg-gradient-to-br from-claw-500 to-purple-600 mb-4 shadow-lg shadow-claw-500/25"
          >
            <span className="text-4xl">🦞</span>
          </motion.div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white mb-2">OpenClaw Manager</h1>
          <p className="text-gray-500 dark:text-gray-400">环境检测与安装向导</p>
        </div>

        <div className="card rounded-2xl p-6 shadow-xl">
          {renderContent()}
        </div>

        <p className="text-center text-gray-400 dark:text-gray-600 text-xs mt-6">
          OpenClaw Manager v0.0.7
        </p>
      </motion.div>
    </div>
  );
}
