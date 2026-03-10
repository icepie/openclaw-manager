import { Activity, Cpu, HardDrive, Clock } from 'lucide-react';
import clsx from 'clsx';

interface ServiceStatus {
  running: boolean;
  pid: number | null;
  port: number;
  uptime_seconds: number | null;
  memory_mb: number | null;
  cpu_percent: number | null;
}

interface StatusCardProps {
  status: ServiceStatus | null;
  loading: boolean;
}

const formatUptime = (seconds: number | null) => {
  if (!seconds) return '--';
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
};

const stats = (status: ServiceStatus | null) => [
  { icon: Activity, label: '端口', value: status?.port || 18789, color: 'text-sky-500' },
  { icon: Cpu, label: '进程 ID', value: status?.pid || '--', color: 'text-violet-500' },
  { icon: HardDrive, label: '内存', value: status?.memory_mb ? `${status.memory_mb.toFixed(0)} MB` : '--', color: 'text-emerald-500' },
  { icon: Clock, label: '运行时间', value: formatUptime(status?.uptime_seconds || null), color: 'text-amber-500' },
];

export function StatusCard({ status, loading }: StatusCardProps) {
  const running = status?.running ?? false;

  return (
    <div className="card p-5">
      {/* 状态行 */}
      <div className="flex items-center justify-between mb-5">
        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">服务状态</span>
        <div className="flex items-center gap-2">
          <div className={clsx('status-dot', loading ? 'warning' : running ? 'running' : 'stopped')} />
          <span className={clsx(
            'text-xs font-medium',
            loading ? 'text-amber-500' : running ? 'text-emerald-500' : 'text-red-500'
          )}>
            {loading ? '检测中' : running ? '运行中' : '已停止'}
          </span>
        </div>
      </div>

      {/* 指标网格 */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {stats(status).map(({ icon: Icon, label, value, color }) => (
          <div key={label} className="flex flex-col gap-1.5">
            <div className="flex items-center gap-1.5">
              <Icon size={13} className={color} />
              <span className="text-[11px] text-gray-400 dark:text-gray-500">{label}</span>
            </div>
            <span className="text-base font-semibold text-gray-900 dark:text-white tabular-nums">
              {String(value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
