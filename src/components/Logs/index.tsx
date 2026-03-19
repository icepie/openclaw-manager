import { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import {
  Trash2,
  RefreshCw,
  Download,
  Filter,
  Terminal,
} from 'lucide-react';
import clsx from 'clsx';
import { logStore, LogEntry } from '../../lib/logger';
import { isTauri } from '../../lib/tauri';

type FilterLevel = 'all' | 'debug' | 'info' | 'warn' | 'error';
type TabType = 'frontend' | 'gateway';

const LEVEL_COLORS: Record<string, string> = {
  debug: 'text-gray-400',
  info: 'text-green-400',
  warn: 'text-yellow-400',
  error: 'text-red-400',
};

const LEVEL_BG: Record<string, string> = {
  debug: 'bg-gray-500/10',
  info: 'bg-green-500/10',
  warn: 'bg-yellow-500/10',
  error: 'bg-red-500/10',
};

const MODULE_COLORS: Record<string, string> = {
  App: 'text-purple-400',
  Service: 'text-blue-400',
  Config: 'text-emerald-400',
  AI: 'text-pink-400',
  Channel: 'text-orange-400',
  Setup: 'text-cyan-400',
  Dashboard: 'text-lime-400',
  Testing: 'text-fuchsia-400',
  API: 'text-amber-400',
};

export function Logs() {
  const [tab, setTab] = useState<TabType>('gateway');
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [gatewayLogs, setGatewayLogs] = useState<string[]>([]);
  const [filter, setFilter] = useState<FilterLevel>('all');
  const [moduleFilter, setModuleFilter] = useState<string>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const logsEndRef = useRef<HTMLDivElement>(null);
  const gatewayEndRef = useRef<HTMLDivElement>(null);

  // 订阅前端日志
  useEffect(() => {
    const updateLogs = () => setLogs(logStore.getAll());
    updateLogs();
    return logStore.subscribe(updateLogs);
  }, []);

  // 轮询 gateway 日志（每秒）
  useEffect(() => {
    if (!isTauri()) return;
    const fetch = async () => {
      try {
        const lines = await invoke<string[]>('get_logs', { lines: 500 });
        setGatewayLogs(lines);
      } catch {}
    };
    fetch();
    const id = setInterval(fetch, 1000);
    return () => clearInterval(id);
  }, []);

  // 自动滚动
  useEffect(() => {
    if (!autoScroll) return;
    if (tab === 'gateway') gatewayEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    else logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs, gatewayLogs, autoScroll, tab]);

  const filteredLogs = logs.filter(log => {
    if (filter !== 'all' && log.level !== filter) return false;
    if (moduleFilter !== 'all' && log.module !== moduleFilter) return false;
    return true;
  });

  const modules = [...new Set(logs.map(log => log.module))];

  const handleClear = () => logStore.clear();

  const handleExport = () => {
    const content = filteredLogs.map(log => {
      const time = log.timestamp.toLocaleTimeString('zh-CN', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
      const args = log.args.length > 0 ? ' ' + JSON.stringify(log.args) : '';
      return `[${time}] [${log.level.toUpperCase()}] [${log.module}] ${log.message}${args}`;
    }).join('\n');
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `openclaw-manager-logs-${new Date().toISOString().slice(0, 10)}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const formatTime = (date: Date) =>
    date.toLocaleTimeString('zh-CN', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
    + '.' + String(date.getMilliseconds()).padStart(3, '0');

  const formatArgs = (args: unknown[]): string => {
    if (args.length === 0) return '';
    try {
      return args.map(arg => typeof arg === 'object' ? JSON.stringify(arg, null, 2) : String(arg)).join(' ');
    } catch {
      return '[无法序列化]';
    }
  };

  return (
    <div className="h-full flex flex-col overflow-hidden gap-3">
      {/* Tab 切换 */}
      <div className="flex items-center gap-1">
        <button
          onClick={() => setTab('gateway')}
          className={clsx('px-3 py-1.5 rounded-lg text-xs font-medium transition-colors', tab === 'gateway' ? 'bg-claw-500/20 text-claw-500' : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300')}
        >
          Gateway 日志
        </button>
        <button
          onClick={() => setTab('frontend')}
          className={clsx('px-3 py-1.5 rounded-lg text-xs font-medium transition-colors', tab === 'frontend' ? 'bg-claw-500/20 text-claw-500' : 'text-gray-500 hover:text-gray-700 dark:hover:text-gray-300')}
        >
          应用日志
        </button>
      </div>

      {tab === 'gateway' ? (
        <div className="flex-1 flex flex-col overflow-hidden gap-2">
          <div className="flex items-center gap-2">
            <label className="flex items-center gap-1 text-[11px] text-gray-400 cursor-pointer">
              <input type="checkbox" checked={autoScroll} onChange={(e) => setAutoScroll(e.target.checked)} className="w-3 h-3 rounded accent-claw-500" />
              自动滚动
            </label>
            <span className="text-[11px] text-gray-400">{gatewayLogs.length} 行</span>
          </div>
          <div className="flex-1 rounded-xl border border-gray-200 dark:border-white/[0.06] overflow-hidden flex flex-col bg-gray-950 dark:bg-black/40">
            <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-800 dark:border-white/[0.06]">
              <Terminal size={12} className="text-gray-500" />
              <span className="text-[11px] text-gray-500 font-medium">~/.openclaw/logs/gateway.log</span>
            </div>
            <div className="flex-1 overflow-y-auto p-2 font-mono text-xs">
              {gatewayLogs.length === 0 ? (
                <div className="h-full flex items-center justify-center text-gray-500">
                  <div className="text-center">
                    <Terminal size={32} className="mx-auto mb-2 opacity-50" />
                    <p>暂无日志</p>
                  </div>
                </div>
              ) : (
                <>
                  {gatewayLogs.map((line, i) => (
                    <div key={i} className="py-0.5 text-gray-300 break-all">{line}</div>
                  ))}
                  <div ref={gatewayEndRef} />
                </>
              )}
            </div>
          </div>
        </div>
      ) : (
        <div className="flex-1 flex flex-col overflow-hidden gap-2">
          {/* 工具栏 */}
          <div className="flex items-center gap-2 flex-wrap">
            <div className="flex items-center gap-1.5">
              <Filter size={13} className="text-gray-400" />
              <select
                value={filter}
                onChange={(e) => setFilter(e.target.value as FilterLevel)}
                className="bg-white dark:bg-white/[0.06] border border-gray-200 dark:border-white/[0.08] rounded-lg px-2.5 py-1.5 text-xs text-gray-700 dark:text-gray-300"
              >
                <option value="all">所有级别</option>
                <option value="debug">Debug</option>
                <option value="info">Info</option>
                <option value="warn">Warn</option>
                <option value="error">Error</option>
              </select>
            </div>
            <select
              value={moduleFilter}
              onChange={(e) => setModuleFilter(e.target.value)}
              className="bg-white dark:bg-white/[0.06] border border-gray-200 dark:border-white/[0.08] rounded-lg px-2.5 py-1.5 text-xs text-gray-700 dark:text-gray-300"
            >
              <option value="all">所有模块</option>
              {modules.map(module => (
                <option key={module} value={module}>{module}</option>
              ))}
            </select>
            <div className="flex items-center gap-2 text-[11px] text-gray-400">
              <span>{filteredLogs.length}/{logs.length}</span>
              <span className="text-red-400">{logs.filter(l => l.level === 'error').length} 错误</span>
              <span className="text-amber-400">{logs.filter(l => l.level === 'warn').length} 警告</span>
            </div>
            <div className="flex items-center gap-1">
              <label className="flex items-center gap-1 text-[11px] text-gray-400 cursor-pointer">
                <input type="checkbox" checked={autoScroll} onChange={(e) => setAutoScroll(e.target.checked)} className="w-3 h-3 rounded accent-claw-500" />
                自动滚动
              </label>
              <button onClick={handleExport} className="icon-btn" title="导出"><Download size={14} /></button>
              <button onClick={() => setLogs(logStore.getAll())} className="icon-btn" title="刷新"><RefreshCw size={14} /></button>
              <button onClick={handleClear} className="icon-btn hover:text-red-500" title="清除"><Trash2 size={14} /></button>
            </div>
          </div>

          {/* 日志列表 */}
          <div className="flex-1 rounded-xl border border-gray-200 dark:border-white/[0.06] overflow-hidden flex flex-col bg-gray-950 dark:bg-black/40">
            <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-800 dark:border-white/[0.06]">
              <Terminal size={12} className="text-gray-500" />
              <span className="text-[11px] text-gray-500 font-medium">应用日志</span>
            </div>
            <div className="flex-1 overflow-y-auto p-2 font-mono text-xs">
              {filteredLogs.length === 0 ? (
                <div className="h-full flex items-center justify-center text-gray-500">
                  <div className="text-center">
                    <Terminal size={32} className="mx-auto mb-2 opacity-50" />
                    <p>暂无日志</p>
                  </div>
                </div>
              ) : (
                <>
                  {filteredLogs.map((log) => (
                    <motion.div
                      key={log.id}
                      initial={{ opacity: 0, x: -10 }}
                      animate={{ opacity: 1, x: 0 }}
                      className={clsx('py-1.5 px-2 rounded mb-1', LEVEL_BG[log.level])}
                    >
                      <div className="flex items-start gap-2">
                        <span className="text-gray-600 flex-shrink-0">{formatTime(log.timestamp)}</span>
                        <span className={clsx('px-1 py-0.5 rounded text-[10px] uppercase flex-shrink-0 font-mono', LEVEL_COLORS[log.level])}>
                          {log.level}
                        </span>
                        <span className={clsx('flex-shrink-0 text-[10px]', MODULE_COLORS[log.module] || 'text-gray-500')}>
                          [{log.module}]
                        </span>
                        <span className="text-gray-300 break-all">{log.message}</span>
                      </div>
                      {log.args.length > 0 && (
                        <div className="mt-1 ml-20 text-gray-500 break-all whitespace-pre-wrap">
                          {formatArgs(log.args)}
                        </div>
                      )}
                    </motion.div>
                  ))}
                  <div ref={logsEndRef} />
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
