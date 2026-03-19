import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Sidebar } from './components/Layout/Sidebar';
import { Header } from './components/Layout/Header';
import { Dashboard } from './components/Dashboard';
import { AIConfig } from './components/AIConfig';
import { Channels } from './components/Channels';
import { Settings } from './components/Settings';
import { Testing } from './components/Testing';
import { Logs } from './components/Logs';
import { appLogger } from './lib/logger';
import { isTauri } from './lib/tauri';

export type PageType = 'dashboard' | 'ai' | 'channels' | 'testing' | 'logs' | 'settings';

export interface EnvironmentStatus {
  node_installed: boolean;
  node_version: string | null;
  node_version_ok: boolean;
  openclaw_installed: boolean;
  openclaw_version: string | null;
  config_dir_exists: boolean;
  ready: boolean;
  os: string;
}

interface ServiceStatus {
  running: boolean;
  pid: number | null;
  port: number;
}

function App() {
  const [currentPage, setCurrentPage] = useState<PageType>('dashboard');
  const [isReady, setIsReady] = useState<boolean | null>(null);
  const [envStatus, setEnvStatus] = useState<EnvironmentStatus | null>(null);
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus | null>(null);

  const [closeDialog, setCloseDialog] = useState(false);
  const [rememberClose, setRememberClose] = useState(false);

  // 关闭行为处理
  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = listen('close-requested', async () => {
      const pref = localStorage.getItem('close_behavior') || 'ask';
      if (pref === 'tray') {
        await getCurrentWindow().hide();
      } else if (pref === 'quit') {
        invoke('force_quit');
      } else {
        setCloseDialog(true);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const handleCloseAction = async (action: 'tray' | 'quit') => {
    if (rememberClose) localStorage.setItem('close_behavior', action);
    setCloseDialog(false);
    if (action === 'tray') {
      await getCurrentWindow().hide();
    } else {
      invoke('force_quit');
    }
  };

  // 检查环境
  const checkEnvironment = useCallback(async () => {
    if (!isTauri()) {
      appLogger.warn('不在 Tauri 环境中，跳过环境检查');
      setIsReady(true);
      return;
    }
    
    appLogger.info('开始检查系统环境...');
    try {
      const status = await invoke<EnvironmentStatus>('check_environment');
      appLogger.info('环境检查完成', status);
      setEnvStatus(status);
      setIsReady(true); // 总是显示主界面
    } catch (e) {
      appLogger.error('环境检查失败', e);
      setIsReady(true);
    }
  }, []);

  useEffect(() => {
    appLogger.info('🦞 App 组件已挂载');
    checkEnvironment();
  }, [checkEnvironment]);

  // 定期获取服务状态
  useEffect(() => {
    // 不在 Tauri 环境中则不轮询
    if (!isTauri()) return;
    
    const fetchServiceStatus = async () => {
      try {
        const status = await invoke<ServiceStatus>('get_service_status');
        setServiceStatus(status);
      } catch {
        // 静默处理轮询错误
      }
    };
    fetchServiceStatus();
    const interval = setInterval(fetchServiceStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  const handleSetupComplete = useCallback(() => {
    appLogger.info('安装向导完成');
    checkEnvironment(); // 重新检查环境
  }, [checkEnvironment]);

  // 页面切换处理
  const handleNavigate = (page: PageType) => {
    appLogger.action('页面切换', { from: currentPage, to: page });
    setCurrentPage(page);
  };

  const renderPage = () => {
    const pages: Record<PageType, JSX.Element> = {
      dashboard: <Dashboard envStatus={envStatus} onSetupComplete={handleSetupComplete} />,
      ai: <AIConfig />,
      channels: <Channels />,
      testing: <Testing />,
      logs: <Logs />,
      settings: <Settings onEnvironmentChange={checkEnvironment} />,
    };

    return (
      <div className="h-full relative">
        {(Object.keys(pages) as PageType[]).map((page) => (
          <div
            key={page}
            className={`h-full absolute inset-0 transition-opacity duration-150 ${
              page === currentPage ? 'opacity-100 pointer-events-auto z-10' : 'opacity-0 pointer-events-none z-0'
            }`}
          >
            {pages[page]}
          </div>
        ))}
      </div>
    );
  };

  // 正在检查环境
  if (isReady === null) {
    return (
      <div className="flex h-screen bg-gray-50 dark:bg-dark-900 items-center justify-center">
        <div className="fixed inset-0 bg-gradient-radial pointer-events-none" />
        <div className="relative z-10 text-center">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-xl bg-gradient-to-br from-claw-400 to-claw-600 mb-4 animate-pulse">
            <span className="text-3xl">🦞</span>
          </div>
          <p className="text-gray-500 dark:text-gray-400">正在启动...</p>
        </div>
      </div>
    );
  }

  // 主界面
  return (
    <div className="flex h-screen bg-gray-50 dark:bg-[#0d0d0f] overflow-hidden">
      {/* 背景装饰 */}
      <div className="fixed inset-0 bg-gradient-radial pointer-events-none" />

      {/* 关闭确认对话框 */}
      {closeDialog && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
          <div className="bg-white dark:bg-dark-800 rounded-2xl border border-gray-200 dark:border-dark-600 p-6 w-80 shadow-xl">
            <h3 className="text-base font-semibold text-gray-900 dark:text-white mb-1">关闭窗口</h3>
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">请选择关闭行为</p>
            <div className="space-y-2 mb-4">
              <button
                onClick={() => handleCloseAction('tray')}
                className="w-full text-left px-4 py-3 rounded-xl bg-gray-50 dark:bg-dark-700 hover:bg-claw-500/10 border border-gray-200 dark:border-dark-500 hover:border-claw-500/40 transition-all"
              >
                <p className="text-sm font-medium text-gray-900 dark:text-white">最小化到托盘</p>
                <p className="text-xs text-gray-500 dark:text-gray-400">后台继续运行</p>
              </button>
              <button
                onClick={() => handleCloseAction('quit')}
                className="w-full text-left px-4 py-3 rounded-xl bg-gray-50 dark:bg-dark-700 hover:bg-red-500/10 border border-gray-200 dark:border-dark-500 hover:border-red-500/40 transition-all"
              >
                <p className="text-sm font-medium text-gray-900 dark:text-white">退出应用</p>
                <p className="text-xs text-gray-500 dark:text-gray-400">完全退出程序</p>
              </button>
            </div>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={rememberClose}
                onChange={e => setRememberClose(e.target.checked)}
                className="rounded"
              />
              <span className="text-xs text-gray-500 dark:text-gray-400">记住我的选择</span>
            </label>
          </div>
        </div>
      )}
      
      {/* 更新提示横幅 */}
      {/* 侧边栏 */}
      <Sidebar currentPage={currentPage} onNavigate={handleNavigate} serviceStatus={serviceStatus} />
      
      {/* 主内容区 */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* 标题栏（macOS 拖拽区域） */}
        <Header currentPage={currentPage} />
        
        {/* 页面内容 */}
        <main className="flex-1 overflow-hidden p-5">
          {renderPage()}
        </main>
      </div>
    </div>
  );
}

export default App;
