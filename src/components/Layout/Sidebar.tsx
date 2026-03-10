import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  LayoutDashboard,
  Bot,
  MessageSquare,
  FlaskConical,
  ScrollText,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react';
import { PageType } from '../../App';
import clsx from 'clsx';

interface ServiceStatus {
  running: boolean;
  pid: number | null;
  port: number;
}

interface SidebarProps {
  currentPage: PageType;
  onNavigate: (page: PageType) => void;
  serviceStatus: ServiceStatus | null;
}

const menuItems: { id: PageType; label: string; icon: React.ElementType }[] = [
  { id: 'dashboard', label: '概览', icon: LayoutDashboard },
  { id: 'ai', label: 'AI 配置', icon: Bot },
  { id: 'channels', label: '消息渠道', icon: MessageSquare },
  { id: 'testing', label: '测试诊断', icon: FlaskConical },
  { id: 'logs', label: '应用日志', icon: ScrollText },
  { id: 'settings', label: '设置', icon: Settings },
];

export function Sidebar({ currentPage, onNavigate, serviceStatus }: SidebarProps) {
  const isRunning = serviceStatus?.running ?? false;
  const [collapsed, setCollapsed] = useState(false);

  return (
    <motion.aside
      animate={{ width: collapsed ? 56 : 220 }}
      transition={{ type: 'spring', stiffness: 320, damping: 32 }}
      className="relative flex-shrink-0 flex flex-col overflow-hidden
        bg-white dark:bg-[#111114]
        border-r border-gray-100 dark:border-white/[0.06]"
    >
      {/* Logo + 折叠按钮 */}
      <div className="h-12 flex items-center justify-between px-3 titlebar-drag flex-shrink-0
        border-b border-gray-100 dark:border-white/[0.06]">
        <div className="flex items-center gap-2.5 titlebar-no-drag overflow-hidden">
          <div className="w-7 h-7 flex-shrink-0 rounded-lg bg-gradient-to-br from-claw-400 to-claw-600 flex items-center justify-center text-base">
            🦞
          </div>
          <AnimatePresence>
            {!collapsed && (
              <motion.span
                initial={{ opacity: 0, width: 0 }}
                animate={{ opacity: 1, width: 'auto' }}
                exit={{ opacity: 0, width: 0 }}
                transition={{ duration: 0.15 }}
                className="text-sm font-semibold text-gray-900 dark:text-white whitespace-nowrap overflow-hidden"
              >
                OpenClaw
              </motion.span>
            )}
          </AnimatePresence>
        </div>
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="titlebar-no-drag flex-shrink-0 icon-btn"
          title={collapsed ? '展开' : '折叠'}
        >
          {collapsed
            ? <PanelLeftOpen size={15} />
            : <PanelLeftClose size={15} />
          }
        </button>
      </div>

      {/* 导航 */}
      <nav className="flex-1 py-2 px-2 overflow-hidden">
        <ul className="space-y-0.5">
          {menuItems.map((item) => {
            const isActive = currentPage === item.id;
            const Icon = item.icon;
            return (
              <li key={item.id}>
                <button
                  onClick={() => onNavigate(item.id)}
                  title={collapsed ? item.label : undefined}
                  className={clsx(
                    'w-full flex items-center rounded-lg text-sm transition-all duration-150 relative',
                    collapsed ? 'justify-center p-2' : 'gap-2.5 px-3 py-2',
                    isActive
                      ? 'bg-gray-100 dark:bg-white/[0.08] text-gray-900 dark:text-white font-medium'
                      : 'text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-white/[0.04] hover:text-gray-900 dark:hover:text-gray-200'
                  )}
                >
                  {isActive && !collapsed && (
                    <motion.div
                      layoutId="activeBar"
                      className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-4 bg-claw-500 rounded-full"
                      transition={{ type: 'spring', stiffness: 320, damping: 32 }}
                    />
                  )}
                  <Icon size={16} className={clsx('flex-shrink-0', isActive ? 'text-claw-500' : '')} />
                  <AnimatePresence>
                    {!collapsed && (
                      <motion.span
                        initial={{ opacity: 0, width: 0 }}
                        animate={{ opacity: 1, width: 'auto' }}
                        exit={{ opacity: 0, width: 0 }}
                        transition={{ duration: 0.12 }}
                        className="overflow-hidden whitespace-nowrap"
                      >
                        {item.label}
                      </motion.span>
                    )}
                  </AnimatePresence>
                </button>
              </li>
            );
          })}
        </ul>
      </nav>

      {/* 底部状态 */}
      <div className="px-2 pb-3 flex-shrink-0">
        {collapsed ? (
          <div className="flex justify-center py-2">
            <div
              className={clsx('status-dot', isRunning ? 'running' : 'stopped')}
              title={isRunning ? '服务运行中' : '服务未启动'}
            />
          </div>
        ) : (
          <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-50 dark:bg-white/[0.03]">
            <div className={clsx('status-dot', isRunning ? 'running' : 'stopped')} />
            <span className="text-xs text-gray-500 dark:text-gray-500 whitespace-nowrap">
              {isRunning ? `运行中 · ${serviceStatus?.port ?? 18789}` : '未启动'}
            </span>
          </div>
        )}
      </div>
    </motion.aside>
  );
}
