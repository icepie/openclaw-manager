import { useEffect, useState } from 'react';
import { Monitor, Package, Folder, CheckCircle, XCircle } from 'lucide-react';
import { api, SystemInfo as SystemInfoType, isTauri } from '../../lib/tauri';

const getOSLabel = (os: string) => ({ macos: 'macOS', windows: 'Windows', linux: 'Linux' }[os] ?? os);

export function SystemInfo() {
  const [info, setInfo] = useState<SystemInfoType | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!isTauri()) { setLoading(false); return; }
    api.getSystemInfo().then(setInfo).catch(() => {}).finally(() => setLoading(false));
  }, []);

  const rows = info ? [
    {
      icon: Monitor,
      iconClass: 'text-sky-500',
      label: '操作系统',
      value: `${getOSLabel(info.os)} ${info.os_version}`,
      sub: info.arch,
    },
    {
      icon: info.openclaw_installed ? CheckCircle : XCircle,
      iconClass: info.openclaw_installed ? 'text-emerald-500' : 'text-red-500',
      label: 'OpenClaw',
      value: info.openclaw_installed ? (info.openclaw_version || '已安装') : '未安装',
    },
    {
      icon: Package,
      iconClass: 'text-emerald-500',
      label: 'Node.js',
      value: info.node_version || '--',
    },
    {
      icon: Folder,
      iconClass: 'text-amber-500',
      label: '配置目录',
      value: info.config_dir || '--',
      mono: true,
    },
  ] : [];

  return (
    <div className="card p-5">
      <span className="text-sm font-medium text-gray-700 dark:text-gray-300 block mb-4">系统信息</span>

      {loading ? (
        <div className="space-y-3 animate-pulse">
          {[60, 80, 50, 90].map((w) => (
            <div key={w} className={`h-3 rounded bg-gray-100 dark:bg-white/[0.06] w-[${w}%]`} />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {rows.map(({ icon: Icon, iconClass, label, value, sub, mono }) => (
            <div key={label} className="flex items-center gap-3">
              <Icon size={14} className={iconClass + ' flex-shrink-0'} />
              <span className="text-xs text-gray-400 dark:text-gray-500 w-16 flex-shrink-0">{label}</span>
              <span className={`text-xs text-gray-700 dark:text-gray-300 truncate ${mono ? 'font-mono' : ''}`}>
                {value}
                {sub && <span className="text-gray-400 dark:text-gray-600 ml-1">({sub})</span>}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
