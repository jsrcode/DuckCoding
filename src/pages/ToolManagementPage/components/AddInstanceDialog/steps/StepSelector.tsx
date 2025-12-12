// 工具和环境选择组件（Step 1）
// 选择工具 + 环境类型 + 添加方式

import { CheckCircle2 } from 'lucide-react';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

const TOOLS = [
  { id: 'claude-code', name: 'Claude Code' },
  { id: 'codex', name: 'CodeX' },
  { id: 'gemini-cli', name: 'Gemini CLI' },
];

const ENV_TYPES = [
  { id: 'local', name: '本地环境', description: '在本机直接运行工具' },
  { id: 'wsl', name: 'WSL 环境', description: 'Windows子系统Linux环境', disabled: true },
  { id: 'ssh', name: 'SSH 远程', description: '远程服务器环境（开发中）', disabled: true },
];

const LOCAL_METHODS = [
  { id: 'auto', name: '自动扫描', description: '自动检测系统中已安装的工具' },
  { id: 'manual', name: '手动指定', description: '选择工具可执行文件路径' },
];

interface StepSelectorProps {
  baseId: string;
  envType: 'local' | 'wsl' | 'ssh';
  localMethod: 'auto' | 'manual';
  onBaseIdChange: (id: string) => void;
  onEnvTypeChange: (type: 'local' | 'wsl' | 'ssh') => void;
  onLocalMethodChange: (method: 'auto' | 'manual') => void;
}

export function StepSelector({
  baseId,
  envType,
  localMethod,
  onBaseIdChange,
  onEnvTypeChange,
  onLocalMethodChange,
}: StepSelectorProps) {
  return (
    <>
      {/* 选择工具 */}
      <div className="space-y-3">
        <Label className="text-base font-semibold">选择工具</Label>
        <div className="grid grid-cols-3 gap-3">
          {TOOLS.map((tool) => (
            <button
              key={tool.id}
              type="button"
              onClick={() => onBaseIdChange(tool.id)}
              className={cn(
                'relative flex items-center justify-center py-2 px-3 rounded-lg border-2 transition-all hover:border-primary/50',
                baseId === tool.id ? 'border-primary bg-primary/5' : 'border-border',
              )}
            >
              {baseId === tool.id && (
                <CheckCircle2 className="absolute top-1 right-1 h-3 w-3 text-primary" />
              )}
              <span className="text-sm font-medium">{tool.name}</span>
            </button>
          ))}
        </div>
      </div>

      {/* 选择环境类型 */}
      <div className="space-y-3">
        <Label className="text-base font-semibold">环境类型</Label>
        <div className="grid grid-cols-3 gap-3">
          {ENV_TYPES.map((env) => (
            <button
              key={env.id}
              type="button"
              onClick={() => !env.disabled && onEnvTypeChange(env.id as 'local' | 'wsl' | 'ssh')}
              disabled={env.disabled}
              className={cn(
                'relative flex flex-col items-center justify-center py-2 px-3 rounded-lg border-2 transition-all',
                env.disabled
                  ? 'opacity-50 cursor-not-allowed'
                  : 'hover:border-primary/50 cursor-pointer',
                envType === env.id && !env.disabled
                  ? 'border-primary bg-primary/5'
                  : 'border-border',
              )}
            >
              {envType === env.id && !env.disabled && (
                <CheckCircle2 className="absolute top-1 right-1 h-3 w-3 text-primary" />
              )}
              <span className="text-sm font-medium mb-1">{env.name}</span>
              <span className="text-xs text-muted-foreground text-center">{env.description}</span>
            </button>
          ))}
        </div>
      </div>

      {/* 本地环境：选择添加方式 */}
      {envType === 'local' && (
        <div className="space-y-3">
          <Label className="text-base font-semibold">添加方式</Label>
          <div className="grid grid-cols-2 gap-3">
            {LOCAL_METHODS.map((method) => (
              <button
                key={method.id}
                type="button"
                onClick={() => onLocalMethodChange(method.id as 'auto' | 'manual')}
                className={cn(
                  'relative flex flex-col items-center justify-center py-2 px-3 rounded-lg border-2 transition-all hover:border-primary/50',
                  localMethod === method.id ? 'border-primary bg-primary/5' : 'border-border',
                )}
              >
                {localMethod === method.id && (
                  <CheckCircle2 className="absolute top-1 right-1 h-3 w-3 text-primary" />
                )}
                <span className="text-sm font-medium mb-1">{method.name}</span>
                <span className="text-xs text-muted-foreground text-center">
                  {method.description}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}
    </>
  );
}
