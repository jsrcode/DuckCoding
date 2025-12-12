// 路径验证组件
// 路径输入框 + 浏览按钮 + 验证状态显示

import { Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';

interface PathValidatorProps {
  value: string;
  validating: boolean;
  error: string | null;
  placeholder: string;
  disabled?: boolean;
  onValueChange: (value: string) => void;
  onBrowse: () => void;
  onValidate?: () => void;
}

export function PathValidator({
  value,
  validating,
  error,
  placeholder,
  disabled = false,
  onValueChange,
  onBrowse,
  onValidate,
}: PathValidatorProps) {
  return (
    <div className="space-y-2">
      <Label>可执行文件路径</Label>
      <div className="flex gap-2">
        <Input
          value={value}
          onChange={(e) => onValueChange(e.target.value)}
          onBlur={onValidate}
          placeholder={placeholder}
          disabled={validating || disabled}
        />
        <Button onClick={onBrowse} variant="outline" disabled={validating || disabled}>
          浏览...
        </Button>
      </div>

      {validating && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          验证中...
        </div>
      )}

      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
    </div>
  );
}
