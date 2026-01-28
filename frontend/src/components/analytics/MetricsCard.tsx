'use client';

import { clsx } from 'clsx';

interface MetricsCardProps {
  label: string;
  value: string | number;
  subValue?: string;
  change?: number; // % change, positive = up, negative = down
  icon?: React.ReactNode;
  variant?: 'default' | 'green' | 'blue' | 'purple' | 'orange';
}

const VARIANT_COLORS = {
  default: 'border-gray-700 bg-gray-800',
  green: 'border-green-800/50 bg-green-900/10',
  blue: 'border-blue-800/50 bg-blue-900/10',
  purple: 'border-purple-800/50 bg-purple-900/10',
  orange: 'border-orange-800/50 bg-orange-900/10',
};

const VARIANT_TEXT = {
  default: 'text-white',
  green: 'text-green-300',
  blue: 'text-blue-300',
  purple: 'text-purple-300',
  orange: 'text-orange-300',
};

export function MetricsCard({
  label,
  value,
  subValue,
  change,
  icon,
  variant = 'default',
}: MetricsCardProps) {
  const isPositive = change !== undefined && change >= 0;
  const changeAbs = change !== undefined ? Math.abs(change) : 0;

  return (
    <div className={clsx('border rounded-xl p-4', VARIANT_COLORS[variant])}>
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <p className="text-xs text-gray-400 font-medium uppercase tracking-wide">{label}</p>
          <p className={clsx('text-2xl font-bold mt-1', VARIANT_TEXT[variant])}>{value}</p>
          {subValue && (
            <p className="text-xs text-gray-500 mt-0.5">{subValue}</p>
          )}
        </div>
        {icon && (
          <div className="text-gray-500 mt-0.5">{icon}</div>
        )}
      </div>

      {change !== undefined && (
        <div className="mt-3 flex items-center gap-1">
          <span
            className={clsx(
              'text-xs font-medium',
              isPositive ? 'text-green-400' : 'text-red-400'
            )}
          >
            {isPositive ? '↑' : '↓'} {changeAbs.toFixed(1)}%
          </span>
          <span className="text-xs text-gray-500">vs last period</span>
        </div>
      )}
    </div>
  );
}
