// WSL 配置组件（Step 2 - WSL）
// 选择 WSL 发行版

import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface WslConfigProps {
  wslDistros: string[];
  selectedDistro: string;
  loadingDistros: boolean;
  onDistroChange: (distro: string) => void;
}

export function WslConfig({
  wslDistros,
  selectedDistro,
  loadingDistros,
  onDistroChange,
}: WslConfigProps) {
  return (
    <div className="space-y-2">
      <Label>选择WSL发行版</Label>
      {loadingDistros ? (
        <div className="rounded border p-3 bg-muted/50 text-sm text-center">加载中...</div>
      ) : wslDistros.length === 0 ? (
        <div className="rounded border p-3 bg-yellow-50 dark:bg-yellow-950/30">
          <p className="text-sm text-yellow-800 dark:text-yellow-200">
            未检测到WSL发行版，请先安装WSL
          </p>
        </div>
      ) : (
        <>
          <Select value={selectedDistro} onValueChange={onDistroChange}>
            <SelectTrigger>
              <SelectValue placeholder="选择发行版" />
            </SelectTrigger>
            <SelectContent>
              {wslDistros.map((distro) => (
                <SelectItem key={distro} value={distro}>
                  {distro}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <div className="rounded border p-3 bg-blue-50 dark:bg-blue-950/30">
            <p className="text-sm text-blue-800 dark:text-blue-200">
              将在 {selectedDistro} 中检测工具安装状态
            </p>
          </div>
        </>
      )}
    </div>
  );
}
