import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import {
  CheckCircle2,
  Loader2,
  Download,
  ArrowRight,
  RefreshCw,
  Package,
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

interface SetupProps {
  onComplete: () => void;
  embedded?: boolean;
}

export function Setup({ onComplete, embedded = false }: SetupProps) {
  const [envStatus, setEnvStatus] = useState<EnvironmentStatus | null>(null);
  const [checking, setChecking] = useState(true);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [step, setStep] = useState<'check' | 'install' | 'complete'>('check');

  const checkEnvironment = async () => {
    setupLogger.info('检查系统环境...');
    setChecking(true);
    setError(null);
    try {
      const status = await invoke<EnvironmentStatus>('check_environment');
      setupLogger.state('环境状态', status);
      setEnvStatus(status);

      if (status.openclaw_installed) {
        setupLogger.info('✅ 环境就绪');
        setStep('complete');
        setTimeout(() => onComplete(), 1500);
      } else {
        setupLogger.warn('OpenClaw 未安装');
        setStep('install');
      }
    } catch (e) {
      setupLogger.error('检查环境失败', e);
      setError(`检查环境失败: ${e}`);
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    setupLogger.info('Setup 组件初始化');
    checkEnvironment();
  }, []);

  const handleInstallOpenclaw = async () => {
    setupLogger.action('安装 OpenClaw');
    setInstalling(true);
    setError(null);

    try {
      const result = await invoke<InstallResult>('install_openclaw');

      if (result.success) {
        setupLogger.info('✅ OpenClaw 安装成功，初始化配置...');
        await invoke<InstallResult>('init_openclaw_config');
        setupLogger.info('✅ 配置初始化完成');
        await checkEnvironment();
      } else {
        setError(result.error || result.message || '安装失败，请重试');
      }
    } catch (e) {
      setError(`安装失败: ${e}`);
    } finally {
      setInstalling(false);
    }
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
        {!checking && step === 'install' && envStatus && (
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
                    安装中...
                  </>
                ) : (
                  <>
                    <Download className="w-4 h-4" />
                    安装
                  </>
                )}
              </button>
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

            {/* 操作按钮 */}
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

  // 嵌入模式
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

  // 全屏模式
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
