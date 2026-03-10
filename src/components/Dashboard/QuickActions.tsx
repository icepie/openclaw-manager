import { Play, Square, RotateCcw } from 'lucide-react';
import clsx from 'clsx';

interface ServiceStatus {
  running: boolean;
  pid: number | null;
  port: number;
}

interface QuickActionsProps {
  status: ServiceStatus | null;
  loading: boolean;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
}

export function QuickActions({ status, loading, onStart, onStop, onRestart }: QuickActionsProps) {
  const running = status?.running ?? false;

  const actions = [
    {
      label: '启动',
      icon: Play,
      onClick: onStart,
      disabled: loading || running,
      activeColor: 'text-emerald-500',
      hoverBg: 'hover:bg-emerald-50 dark:hover:bg-emerald-500/10 hover:border-emerald-200 dark:hover:border-emerald-500/20',
    },
    {
      label: '停止',
      icon: Square,
      onClick: onStop,
      disabled: loading || !running,
      activeColor: 'text-red-500',
      hoverBg: 'hover:bg-red-50 dark:hover:bg-red-500/10 hover:border-red-200 dark:hover:border-red-500/20',
    },
    {
      label: '重启',
      icon: RotateCcw,
      onClick: onRestart,
      disabled: loading,
      activeColor: 'text-amber-500',
      hoverBg: 'hover:bg-amber-50 dark:hover:bg-amber-500/10 hover:border-amber-200 dark:hover:border-amber-500/20',
      spin: loading,
    },
  ];

  return (
    <div className="card p-5">
      <span className="text-sm font-medium text-gray-700 dark:text-gray-300 block mb-4">快捷操作</span>
      <div className="flex gap-3">
        {actions.map(({ label, icon: Icon, onClick, disabled, activeColor, hoverBg, spin }) => (
          <button
            key={label}
            onClick={onClick}
            disabled={disabled}
            className={clsx(
              'flex-1 flex flex-col items-center gap-2 py-4 rounded-xl border transition-all duration-150',
              'border-gray-100 dark:border-white/[0.06]',
              'bg-gray-50/50 dark:bg-white/[0.02]',
              disabled ? 'opacity-40 cursor-not-allowed' : hoverBg
            )}
          >
            <Icon
              size={18}
              className={clsx(
                'transition-colors',
                disabled ? 'text-gray-400 dark:text-gray-600' : activeColor,
                spin && 'animate-spin'
              )}
            />
            <span className={clsx(
              'text-xs font-medium',
              disabled ? 'text-gray-400 dark:text-gray-600' : 'text-gray-600 dark:text-gray-400'
            )}>
              {label}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
