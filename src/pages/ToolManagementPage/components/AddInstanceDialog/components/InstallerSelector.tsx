// 安装器选择器组件
// 下拉选择安装器 或 显示安装器类型按钮组 + 自定义路径输入

import { CheckCircle2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { InfoIcon } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { InstallerCandidate } from '@/lib/tauri-commands';

const INSTALL_METHODS = [
  { id: 'npm', name: 'npm', description: '使用 npm 安装' },
  { id: 'brew', name: 'Homebrew', description: '使用 brew 安装（仅 macOS）' },
  { id: 'official', name: '官方脚本', description: '使用官方安装脚本' },
  { id: 'other', name: '其他', description: '不支持APP内快捷更新' },
];

interface InstallerSelectorProps {
  installerCandidates: InstallerCandidate[];
  selectedPath: string;
  installMethod: string;
  showCustomMode: boolean;
  disabled?: boolean;
  onInstallerSelect: (path: string) => void;
  onInstallMethodChange: (method: string) => void;
  onCustomModeToggle: () => void;
  onBrowse: () => void;
}

export function InstallerSelector({
  installerCandidates,
  selectedPath,
  installMethod,
  showCustomMode,
  disabled = false,
  onInstallerSelect,
  onInstallMethodChange,
  onCustomModeToggle,
  onBrowse,
}: InstallerSelectorProps) {
  // 情况A：扫描到安装器且未点击自定义 - 显示下拉选择
  if (installerCandidates.length > 0 && !showCustomMode) {
    return (
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label>安装器路径</Label>
          <Button variant="ghost" size="sm" onClick={onCustomModeToggle} disabled={disabled}>
            自定义
          </Button>
        </div>
        <Select value={selectedPath} onValueChange={onInstallerSelect} disabled={disabled}>
          <SelectTrigger>
            <SelectValue placeholder="选择安装器" />
          </SelectTrigger>
          <SelectContent>
            {installerCandidates.map((candidate, index) => (
              <SelectItem key={index} value={candidate.path}>
                {candidate.path} ({candidate.installer_type})
                {candidate.level === 1 ? ' [同级]' : ' [上级]'}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          已自动扫描到 {installerCandidates.length} 个安装器，点击「自定义」可手动配置
        </p>
      </div>
    );
  }

  // 情况B：点击自定义 或 没扫描到 - 显示安装器类型和路径输入
  return (
    <>
      <div className="space-y-3">
        <Label className="text-base font-semibold">安装器类型</Label>
        <div className="grid grid-cols-4 gap-2">
          {INSTALL_METHODS.map((method) => (
            <button
              key={method.id}
              type="button"
              onClick={() => onInstallMethodChange(method.id)}
              disabled={disabled}
              className={cn(
                'relative flex flex-col items-center justify-center py-2 px-2 rounded-lg border-2 transition-all hover:border-primary/50',
                installMethod === method.id ? 'border-primary bg-primary/5' : 'border-border',
                disabled && 'opacity-50 cursor-not-allowed',
              )}
            >
              {installMethod === method.id && (
                <CheckCircle2 className="absolute top-1 right-1 h-3 w-3 text-primary" />
              )}
              <span className="text-xs font-medium mb-0.5">{method.name}</span>
              <span className="text-[10px] text-muted-foreground text-center leading-tight">
                {method.description}
              </span>
            </button>
          ))}
        </div>
      </div>

      {/* 安装器路径输入（非 other 时显示） */}
      {installMethod !== 'other' && (
        <div className="space-y-2">
          <Label>安装器路径（用于更新工具）</Label>
          <div className="flex gap-2">
            <Input
              value={selectedPath}
              onChange={(e) => onInstallerSelect(e.target.value)}
              placeholder={`如: ${navigator.platform.toLowerCase().includes('win') ? 'C:\\Program Files\\nodejs\\npm.cmd' : '/usr/local/bin/npm'}`}
              disabled={disabled}
            />
            <Button onClick={onBrowse} variant="outline" disabled={disabled}>
              浏览...
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            {installerCandidates.length === 0
              ? '未检测到安装器，请手动选择或留空（无法快捷更新）'
              : '手动指定安装器路径'}
          </p>
        </div>
      )}

      {/* Other 类型警告 */}
      {installMethod === 'other' && (
        <Alert variant="default" className="border-yellow-500 bg-yellow-50 dark:bg-yellow-950/30">
          <InfoIcon className="h-4 w-4 text-yellow-600" />
          <AlertDescription className="text-yellow-800 dark:text-yellow-200">
            <strong>「其他」类型不支持 APP 内快捷更新</strong>
            <br />
            您需要手动更新此工具
          </AlertDescription>
        </Alert>
      )}
    </>
  );
}
