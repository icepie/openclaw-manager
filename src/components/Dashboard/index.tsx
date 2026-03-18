import { useEffect, useState, useRef } from 'react';
import { motion } from 'framer-motion';
import { invoke } from '@tauri-apps/api/core';
import { StatusCard } from './StatusCard';
import { QuickActions } from './QuickActions';
import { SystemInfo } from './SystemInfo';
import { Setup } from '../Setup';
import { api, ServiceStatus, isTauri } from '../../lib/tauri';
import { Terminal, RefreshCw, ChevronDown, ChevronUp } from 'lucide-react';
import clsx from 'clsx';
import { EnvironmentStatus } from '../../App';

interface DashboardProps {
  envStatus: EnvironmentStatus | null;
  onSetupComplete: () => void;
}

export function Dashboard({ envStatus, onSetupComplete }: DashboardProps) {
  const [status, setStatus] = useState<ServiceStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const [logsExpanded, setLogsExpanded] = useState(true);
  const [autoRefreshLogs, setAutoRefreshLogs] = useState(true);
  const logsContainerRef = useRef<HTMLDivElement>(null);

  const fetchStatus = async () => {
    if (!isTauri()) {
      setLoading(false);
      return;
    }
    try {
      const result = await api.getServiceStatus();
      setStatus(result);
    } catch {
      // 静默处理
    } finally {
      setLoading(false);
    }
  };

  const fetchLogs = async () => {
    if (!isTauri()) return;
    try {
      const result = await invoke<string[]>('get_logs', { lines: 50 });
      setLogs(result);
    } catch {
      // 静默处理
    }
  };

  useEffect(() => {
    fetchStatus();
    fetchLogs();
    if (!isTauri()) return;
    
    const statusInterval = setInterval(fetchStatus, 3000);
    const logsInterval = autoRefreshLogs ? setInterval(fetchLogs, 2000) : null;
    
    return () => {
      clearInterval(statusInterval);
      if (logsInterval) clearInterval(logsInterval);
    };
  }, [autoRefreshLogs]);

  // 自动滚动到日志底部（仅在日志容器内部滚动，不影响页面）
  useEffect(() => {
    if (logsExpanded && logsContainerRef.current) {
      logsContainerRef.current.scrollTop = logsContainerRef.current.scrollHeight;
    }
  }, [logs, logsExpanded]);

  const handleStart = async () => {
    if (!isTauri()) return;
    setActionLoading(true);
    try {
      await api.startService();
      await fetchStatus();
      await fetchLogs();
    } catch (e) {
      console.error('启动失败:', e);
    } finally {
      setActionLoading(false);
    }
  };

  const handleStop = async () => {
    if (!isTauri()) return;
    setActionLoading(true);
    try {
      await api.stopService();
      await fetchStatus();
      await fetchLogs();
    } catch (e) {
      console.error('停止失败:', e);
    } finally {
      setActionLoading(false);
    }
  };

  const handleRestart = async () => {
    if (!isTauri()) return;
    setActionLoading(true);
    try {
      await api.restartService();
      await fetchStatus();
      await fetchLogs();
    } catch (e) {
      console.error('重启失败:', e);
    } finally {
      setActionLoading(false);
    }
  };

  const handleOpenTerminal = async () => {
    if (!isTauri()) return;
    try {
      await invoke('open_env_terminal');
    } catch (e) {
      console.error('打开终端失败:', e);
    }
  };

  const getLogLineClass = (line: string) => {
    if (line.includes('error') || line.includes('Error') || line.includes('ERROR')) {
      return 'text-red-400';
    }
    if (line.includes('warn') || line.includes('Warn') || line.includes('WARN')) {
      return 'text-yellow-400';
    }
    if (line.includes('info') || line.includes('Info') || line.includes('INFO')) {
      return 'text-green-400';
    }
    return 'text-gray-400';
  };

  const containerVariants = {
    hidden: { opacity: 0 },
    show: {
      opacity: 1,
      transition: {
        staggerChildren: 0.1,
      },
    },
  };

  const itemVariants = {
    hidden: { opacity: 0, y: 20 },
    show: { opacity: 1, y: 0 },
  };

  // 检查环境是否就绪
  const needsSetup = envStatus && !envStatus.ready;

  return (
    <div className="h-full overflow-y-auto scroll-container">
      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="show"
        className="space-y-4 max-w-3xl"
      >
        {/* 环境安装向导 */}
        {needsSetup && (
          <motion.div variants={itemVariants}>
            <Setup onComplete={onSetupComplete} embedded />
          </motion.div>
        )}

        {/* 状态 + 操作 两列 */}
        <motion.div variants={itemVariants} className="grid grid-cols-1 sm:grid-cols-5 gap-4">
          <div className="sm:col-span-3">
            <StatusCard status={status} loading={loading} />
          </div>
          <div className="sm:col-span-2">
            <QuickActions
              status={status}
              loading={actionLoading}
              onStart={handleStart}
              onStop={handleStop}
              onRestart={handleRestart}
              onOpenTerminal={handleOpenTerminal}
            />
          </div>
        </motion.div>

        {/* 实时日志 */}
        <motion.div variants={itemVariants}>
          <div className="card overflow-hidden">
            <div
              className="flex items-center justify-between px-4 py-2.5 cursor-pointer
                border-b border-gray-100 dark:border-white/[0.06]
                hover:bg-gray-50 dark:hover:bg-white/[0.02] transition-colors"
              onClick={() => setLogsExpanded(!logsExpanded)}
            >
              <div className="flex items-center gap-2">
                <Terminal size={13} className="text-gray-400" />
                <span className="text-xs font-medium text-gray-700 dark:text-gray-300">实时日志</span>
                <span className="text-[11px] text-gray-400 dark:text-gray-600 tabular-nums">
                  {logs.length} 行
                </span>
              </div>
              <div className="flex items-center gap-2">
                {logsExpanded && (
                  <>
                    <label
                      className="flex items-center gap-1.5 text-[11px] text-gray-400 cursor-pointer"
                      onClick={e => e.stopPropagation()}
                    >
                      <input
                        type="checkbox"
                        checked={autoRefreshLogs}
                        onChange={e => setAutoRefreshLogs(e.target.checked)}
                        className="w-3 h-3 rounded accent-claw-500"
                      />
                      自动刷新
                    </label>
                    <button
                      onClick={e => { e.stopPropagation(); fetchLogs(); }}
                      className="icon-btn p-1"
                    >
                      <RefreshCw size={12} />
                    </button>
                  </>
                )}
                {logsExpanded
                  ? <ChevronUp size={13} className="text-gray-400" />
                  : <ChevronDown size={13} className="text-gray-400" />
                }
              </div>
            </div>

            {logsExpanded && (
              <div
                ref={logsContainerRef}
                className="h-56 overflow-y-auto p-3 font-mono text-[11px] leading-relaxed
                  bg-gray-950 dark:bg-black/40"
              >
                {logs.length === 0 ? (
                  <div className="h-full flex items-center justify-center text-gray-600">
                    暂无日志，请先启动服务
                  </div>
                ) : (
                  logs.map((line, i) => (
                    <div key={i} className={clsx('py-px whitespace-pre-wrap break-all', getLogLineClass(line))}>
                      {line}
                    </div>
                  ))
                )}
              </div>
            )}
          </div>
        </motion.div>

        {/* 系统信息 */}
        <motion.div variants={itemVariants}>
          <SystemInfo />
        </motion.div>
      </motion.div>
    </div>
  );
}
