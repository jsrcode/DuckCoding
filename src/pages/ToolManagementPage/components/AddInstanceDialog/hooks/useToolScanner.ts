// 工具扫描逻辑 Hook
// 封装工具检测和验证的业务逻辑

import { useCallback } from 'react';
import { scanAllToolCandidates, validateToolPath, type ToolCandidate } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';

export interface UseToolScannerParams {
  onCandidatesFound: (candidates: ToolCandidate[]) => void;
  onCandidateSelected: (candidate: ToolCandidate) => void;
  onScanStart: () => void;
  onScanEnd: () => void;
  onValidationStart: () => void;
  onValidationEnd: () => void;
  onValidationError: (error: string) => void;
  onValidationSuccess: (version: string) => void;
}

export function useToolScanner(params: UseToolScannerParams) {
  const { toast } = useToast();

  /**
   * 扫描所有工具候选
   */
  const scanAllCandidates = useCallback(
    async (toolId: string, toolName: string) => {
      console.log('[useToolScanner] 开始扫描，工具:', toolId);
      params.onScanStart();

      try {
        const candidates = await scanAllToolCandidates(toolId);
        console.log('[useToolScanner] 扫描到', candidates.length, '个工具候选');

        params.onCandidatesFound(candidates);

        if (candidates.length === 0) {
          toast({
            variant: 'destructive',
            title: '未检测到工具',
            description: `未在系统中检测到 ${toolName}`,
          });
        } else {
          toast({
            title: '扫描完成',
            description: `找到 ${candidates.length} 个 ${toolName} 实例`,
          });

          // 如果只有一个候选，自动选择
          if (candidates.length === 1) {
            params.onCandidateSelected(candidates[0]);
          }
        }
      } catch (error) {
        console.error('[useToolScanner] 扫描失败:', error);
        toast({
          variant: 'destructive',
          title: '扫描失败',
          description: String(error),
        });
      } finally {
        params.onScanEnd();
      }
    },
    [params, toast],
  );

  /**
   * 验证工具路径
   */
  const validatePath = useCallback(
    async (toolId: string, path: string) => {
      if (!path.trim()) {
        params.onValidationError('请输入路径');
        return;
      }

      console.log('[useToolScanner] 验证路径:', path);
      params.onValidationStart();

      try {
        const version = await validateToolPath(toolId, path);
        console.log('[useToolScanner] 验证成功，版本:', version);
        params.onValidationSuccess(version);
        params.onValidationError(null);
      } catch (error) {
        console.error('[useToolScanner] 验证失败:', error);
        params.onValidationError(String(error));
      } finally {
        params.onValidationEnd();
      }
    },
    [params],
  );

  /**
   * 选择候选
   */
  const selectCandidate = useCallback(
    (candidate: ToolCandidate) => {
      params.onCandidateSelected(candidate);
    },
    [params],
  );

  return {
    scanAllCandidates,
    validatePath,
    selectCandidate,
  };
}
