// 自动扫描配置组件（Step 2 - Auto）
// 扫描系统中的工具实例，展示候选列表

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, InfoIcon } from 'lucide-react';
import type { ToolCandidate } from '@/lib/tauri-commands';
import { ToolCandidateCard } from '../components/ToolCandidateCard';

interface LocalAutoConfigProps {
  toolName: string;
  scanning: boolean;
  candidates: ToolCandidate[];
  selectedCandidate: ToolCandidate | null;
  onScan: () => void;
  onSelectCandidate: (candidate: ToolCandidate) => void;
}

export function LocalAutoConfig({
  toolName,
  scanning,
  candidates,
  selectedCandidate,
  onScan,
  onSelectCandidate,
}: LocalAutoConfigProps) {
  return (
    <>
      <Alert>
        <InfoIcon className="h-4 w-4" />
        <AlertDescription>
          将自动扫描系统中已安装的 {toolName}，包括 npm、Homebrew 等安装方式
        </AlertDescription>
      </Alert>

      <div className="space-y-2">
        <Button onClick={onScan} disabled={scanning} className="w-full" variant="outline">
          {scanning ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              扫描中...
            </>
          ) : (
            '开始扫描'
          )}
        </Button>

        {/* 显示候选列表（多个结果时） */}
        {candidates.length > 1 && (
          <div className="space-y-2">
            <Label>选择工具实例（共 {candidates.length} 个）</Label>
            <div className="space-y-2 max-h-60 overflow-y-auto">
              {candidates.map((candidate, index) => (
                <ToolCandidateCard
                  key={index}
                  candidate={candidate}
                  selected={selectedCandidate === candidate}
                  onClick={() => onSelectCandidate(candidate)}
                />
              ))}
            </div>
          </div>
        )}

        {/* 单个候选时直接显示 */}
        {candidates.length === 1 && selectedCandidate && (
          <Alert>
            <InfoIcon className="h-4 w-4" />
            <AlertDescription>
              <div className="space-y-1">
                <div>✓ 路径：{selectedCandidate.tool_path}</div>
                <div>版本：{selectedCandidate.version}</div>
                <div>安装器：{selectedCandidate.installer_path || '未检测到'}</div>
              </div>
            </AlertDescription>
          </Alert>
        )}
      </div>
    </>
  );
}
