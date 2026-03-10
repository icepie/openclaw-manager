import { useState } from 'react';
import { PageType } from '../../App';
import { RefreshCw, ExternalLink, Loader2, Sun, Moon } from 'lucide-react';
import { open } from '@tauri-apps/plugin-shell';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/appStore';

interface HeaderProps {
  currentPage: PageType;
}

const pageTitles: Record<PageType, { title: string; description: string }> = {
  dashboard: { title: '概览', description: '服务状态与快捷操作' },
  ai: { title: 'AI 配置', description: '配置 AI 提供商和模型' },
  channels: { title: '消息渠道', description: '配置 Telegram、Discord、飞书等' },
  testing: { title: '测试诊断', description: '系统诊断与问题排查' },
  logs: { title: '应用日志', description: '查看 Manager 控制台日志' },
  settings: { title: '设置', description: '身份配置与高级选项' },
};

export function Header({ currentPage }: HeaderProps) {
  const { title, description } = pageTitles[currentPage];
  const [opening, setOpening] = useState(false);
  const { theme, toggleTheme } = useAppStore();

  const handleOpenDashboard = async () => {
    setOpening(true);
    try {
      const url = await invoke<string>('get_dashboard_url');
      await open(url);
    } catch {
      window.open('http://localhost:18789', '_blank');
    } finally {
      setOpening(false);
    }
  };

  return (
    <header className="h-12 flex items-center justify-between px-5 titlebar-drag flex-shrink-0
      border-b border-gray-100 dark:border-white/[0.06]
      bg-white/80 dark:bg-[#111114]/80 backdrop-blur-sm">

      <div className="titlebar-no-drag">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold text-gray-900 dark:text-white">{title}</h2>
          <span className="text-gray-300 dark:text-white/20 text-xs">·</span>
          <p className="text-xs text-gray-400 dark:text-gray-500">{description}</p>
        </div>
      </div>

      <div className="flex items-center gap-1 titlebar-no-drag">
        <button onClick={toggleTheme} className="icon-btn" title={theme === 'dark' ? '日间模式' : '夜间模式'}>
          {theme === 'dark' ? <Sun size={15} /> : <Moon size={15} />}
        </button>
        <button onClick={() => window.location.reload()} className="icon-btn" title="刷新">
          <RefreshCw size={15} />
        </button>
        <button
          onClick={handleOpenDashboard}
          disabled={opening}
          className="btn-ghost ml-1 text-xs"
        >
          {opening ? <Loader2 size={13} className="animate-spin" /> : <ExternalLink size={13} />}
          Dashboard
        </button>
      </div>
    </header>
  );
}
