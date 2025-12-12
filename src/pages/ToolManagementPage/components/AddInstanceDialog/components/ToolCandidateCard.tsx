// 工具候选卡片组件
// 展示单个工具候选的信息（路径、版本、安装器、方法）

import { CheckCircle2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ToolCandidate } from '@/lib/tauri-commands';

interface ToolCandidateCardProps {
  candidate: ToolCandidate;
  selected: boolean;
  onClick: () => void;
}

export function ToolCandidateCard({ candidate, selected, onClick }: ToolCandidateCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-full p-3 rounded-lg border-2 text-left transition-all hover:border-primary/50',
        selected ? 'border-primary bg-primary/5' : 'border-border',
      )}
    >
      <div className="flex items-start justify-between">
        <div className="space-y-1 flex-1">
          <div className="text-sm font-medium">{candidate.tool_path}</div>
          <div className="text-xs text-muted-foreground">版本：{candidate.version}</div>
          <div className="text-xs text-muted-foreground">
            安装器：{candidate.installer_path || '未检测到'}
          </div>
          <div className="text-xs text-muted-foreground">方法：{candidate.install_method}</div>
        </div>
        {selected && <CheckCircle2 className="h-5 w-5 text-primary flex-shrink-0" />}
      </div>
    </button>
  );
}
