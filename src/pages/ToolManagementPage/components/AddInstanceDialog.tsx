import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { useState, useEffect, useCallback } from 'react';
import { Loader2, InfoIcon, CheckCircle2 } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { SSHConfig } from '@/types/tool-management';
import {
  listWslDistributions,
  validateToolPath,
  addManualToolInstance,
  scanInstallerForToolPath,
  scanAllToolCandidates,
  type InstallerCandidate,
  type ToolCandidate,
} from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { cn } from '@/lib/utils';

interface AddInstanceDialogProps {
  open: boolean;
  onClose: () => void;
  onAdd: (
    baseId: string,
    type: 'local' | 'wsl' | 'ssh',
    sshConfig?: SSHConfig,
    distroName?: string,
  ) => Promise<void>;
}

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

const INSTALL_METHODS = [
  { id: 'npm', name: 'npm', description: '使用 npm 安装' },
  { id: 'brew', name: 'Homebrew', description: '使用 brew 安装（仅 macOS）' },
  { id: 'official', name: '官方脚本', description: '使用官方安装脚本' },
  { id: 'other', name: '其他', description: '不支持APP内快捷更新' },
];

export function AddInstanceDialog({ open, onClose, onAdd }: AddInstanceDialogProps) {
  const { toast } = useToast();
  const [step, setStep] = useState(1); // 当前步骤：1=选择工具和方式，2=配置详情
  const [baseId, setBaseId] = useState('claude-code');
  const [envType, setEnvType] = useState<'local' | 'wsl' | 'ssh'>('local');
  const [localMethod, setLocalMethod] = useState<'auto' | 'manual'>('auto');
  const [manualPath, setManualPath] = useState('');
  const [installMethod, setInstallMethod] = useState<'npm' | 'brew' | 'official' | 'other'>('npm');
  const [installerPath, setInstallerPath] = useState('');
  const [installerCandidates, setInstallerCandidates] = useState<InstallerCandidate[]>([]);
  const [toolCandidates, setToolCandidates] = useState<ToolCandidate[]>([]);
  const [selectedToolCandidate, setSelectedToolCandidate] = useState<ToolCandidate | null>(null);
  const [showCustomInstaller, setShowCustomInstaller] = useState(false);
  const [validating, setValidating] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanResult, setScanResult] = useState<{ installed: boolean; version: string } | null>(
    null,
  );
  const [loading, setLoading] = useState(false);
  const [wslDistros, setWslDistros] = useState<string[]>([]);
  const [selectedDistro, setSelectedDistro] = useState<string>('');
  const [loadingDistros, setLoadingDistros] = useState(false);

  const toolNames: Record<string, string> = {
    'claude-code': 'Claude Code',
    codex: 'CodeX',
    'gemini-cli': 'Gemini CLI',
  };

  const loadWslDistros = useCallback(async () => {
    setLoadingDistros(true);
    try {
      const distros = await listWslDistributions();
      setWslDistros(distros);
      if (distros.length > 0) {
        setSelectedDistro(distros[0]);
      }
    } catch (err) {
      toast({
        title: '加载WSL发行版失败',
        description: String(err),
        variant: 'destructive',
      });
      setWslDistros([]);
    } finally {
      setLoadingDistros(false);
    }
  }, [toast]);

  useEffect(() => {
    if (open && envType === 'wsl') {
      loadWslDistros();
    }
  }, [open, envType, loadWslDistros]);

  // 重置扫描结果：当用户更改工具、环境类型或添加方式时
  useEffect(() => {
    setScanResult(null);
    setToolCandidates([]);
    setSelectedToolCandidate(null);
    setInstallerCandidates([]);
  }, [baseId, envType, localMethod]);

  const getCommonPaths = () => {
    const isWindows = navigator.platform.toLowerCase().includes('win');
    if (isWindows) {
      return [
        `C:\\Users\\用户名\\AppData\\Roaming\\npm\\${baseId}.cmd`,
        `C:\\Users\\用户名\\.npm-global\\${baseId}.cmd`,
        `C:\\Program Files\\${toolNames[baseId]}\\${baseId}.exe`,
      ];
    } else {
      return [
        `~/.npm-global/bin/${baseId}`,
        `/usr/local/bin/${baseId}`,
        `/opt/homebrew/bin/${baseId}`,
        `~/.local/bin/${baseId}`,
      ];
    }
  };

  const handleBrowse = async () => {
    try {
      const isWindows = navigator.platform.toLowerCase().includes('win');
      const selected = await openDialog({
        directory: false,
        multiple: false,
        title: `选择 ${toolNames[baseId]} 可执行文件`,
        filters: [
          {
            name: '可执行文件',
            extensions: isWindows ? ['exe', 'cmd', 'bat'] : [],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        setManualPath(selected);
        handleValidate(selected);
      }
    } catch (error) {
      toast({
        variant: 'destructive',
        title: '打开文件选择器失败',
        description: String(error),
      });
    }
  };

  // 浏览选择安装器路径
  const handleBrowseInstaller = async () => {
    try {
      const isWindows = navigator.platform.toLowerCase().includes('win');
      const selected = await openDialog({
        directory: false,
        multiple: false,
        title: `选择安装器可执行文件（${installMethod}）`,
        filters: [
          {
            name: '可执行文件',
            extensions: isWindows ? ['exe', 'cmd', 'bat'] : [],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        setInstallerPath(selected);
      }
    } catch (error) {
      toast({
        variant: 'destructive',
        title: '打开文件选择器失败',
        description: String(error),
      });
    }
  };

  const handleValidate = async (pathToValidate: string) => {
    if (!pathToValidate.trim()) {
      setValidationError('请输入路径');
      return;
    }

    setValidating(true);
    setValidationError(null);

    try {
      await validateToolPath(baseId, pathToValidate);
    } catch (error) {
      setValidationError(String(error));
    } finally {
      setValidating(false);
    }
  };

  // 执行扫描/验证（不保存）
  const handleScan = async () => {
    if (envType !== 'local') return;

    console.log('[AddInstance] 开始扫描，工具:', baseId, '方式:', localMethod);
    setScanning(true);
    setScanResult(null);

    try {
      if (localMethod === 'auto') {
        // 自动扫描（扫描所有可能的工具实例）
        console.log('[AddInstance] 调用 scanAllToolCandidates，工具:', baseId);
        const candidates = await scanAllToolCandidates(baseId);
        console.log('[AddInstance] 扫描到', candidates.length, '个工具候选');
        setToolCandidates(candidates);

        if (candidates.length === 0) {
          toast({
            variant: 'destructive',
            title: '未检测到工具',
            description: `未在系统中检测到 ${toolNames[baseId]}`,
          });
        } else {
          toast({
            title: '扫描完成',
            description: `找到 ${candidates.length} 个 ${toolNames[baseId]} 实例`,
          });

          // 如果只有一个候选，自动选择
          if (candidates.length === 1) {
            setSelectedToolCandidate(candidates[0]);
            setScanResult({ installed: true, version: candidates[0].version });
          }
        }
      } else {
        // 手动验证路径（不保存）并扫描安装器
        if (!manualPath) {
          toast({
            variant: 'destructive',
            title: '请选择路径',
          });
          return;
        }
        if (validationError) {
          toast({
            variant: 'destructive',
            title: '路径验证失败',
            description: validationError,
          });
          return;
        }

        console.log('[AddInstance] 验证路径:', manualPath);
        const version = await validateToolPath(baseId, manualPath);
        console.log('[AddInstance] 验证结果:', version);
        setScanResult({ installed: true, version });

        // 扫描安装器路径
        console.log('[AddInstance] 扫描安装器路径');
        try {
          const installerResults = await scanInstallerForToolPath(manualPath);
          console.log('[AddInstance] 扫描到', installerResults.length, '个安装器候选');
          setInstallerCandidates(installerResults);

          // 自动选择第一个候选
          if (installerResults.length > 0) {
            setInstallerPath(installerResults[0].path);
            // 根据安装器类型设置 installMethod
            const installerType = installerResults[0].installer_type.toLowerCase();
            if (installerType.includes('npm')) {
              setInstallMethod('npm');
            } else if (installerType.includes('brew')) {
              setInstallMethod('brew');
            }
          }
        } catch (error) {
          console.error('[AddInstance] 扫描安装器失败:', error);
          setInstallerCandidates([]);
        }

        toast({
          title: '验证成功',
          description: `${toolNames[baseId]} v${version}`,
        });
      }
    } catch (error) {
      console.error('[AddInstance] 扫描/验证失败:', error);
      toast({
        variant: 'destructive',
        title: '扫描失败',
        description: String(error),
      });
      setScanResult(null);
    } finally {
      setScanning(false);
    }
  };

  const handleSubmit = async () => {
    if (envType === 'local') {
      // 本地环境：保存已扫描的实例
      if (!scanResult || !scanResult.installed) {
        toast({
          variant: 'destructive',
          title: '无可用结果',
          description: '请先执行扫描',
        });
        return;
      }

      setLoading(true);
      try {
        if (localMethod === 'auto') {
          // 自动扫描：使用选中的候选
          if (!selectedToolCandidate) {
            toast({
              variant: 'destructive',
              title: '请选择工具实例',
              description: '请从扫描结果中选择一个实例',
            });
            return;
          }

          console.log(
            '[AddInstance] 保存自动扫描结果，工具:',
            baseId,
            '候选:',
            selectedToolCandidate,
          );

          // 确定安装方法字符串
          const methodStr = selectedToolCandidate.install_method.toLowerCase();

          await addManualToolInstance(
            baseId,
            selectedToolCandidate.tool_path,
            methodStr,
            selectedToolCandidate.installer_path || undefined,
          );

          toast({
            title: '添加成功',
            description: `${toolNames[baseId]} v${selectedToolCandidate.version}`,
          });
        } else {
          // 手动指定：验证并保存路径
          console.log('[AddInstance] 保存手动指定路径:', manualPath, '安装器:', installMethod);

          // 验证：非 other 类型必须提供安装器路径
          if (installMethod !== 'other' && !installerPath) {
            toast({
              variant: 'destructive',
              title: '请选择安装器路径',
              description: `${INSTALL_METHODS.find((m) => m.id === installMethod)?.name} 需要提供安装器路径`,
            });
            return;
          }

          await addManualToolInstance(
            baseId,
            manualPath,
            installMethod,
            installerPath || undefined,
          );
          toast({
            title: '添加成功',
            description: `${toolNames[baseId]} 已成功添加`,
          });
        }

        await onAdd(baseId, 'local');
        handleClose();
      } catch (error) {
        toast({
          variant: 'destructive',
          title: '添加失败',
          description: String(error),
        });
      } finally {
        setLoading(false);
      }
    } else if (envType === 'ssh') {
      return;
    } else if (envType === 'wsl') {
      if (!selectedDistro) {
        toast({
          title: '请选择WSL发行版',
          variant: 'destructive',
        });
        return;
      }

      setLoading(true);
      try {
        await onAdd(baseId, envType, undefined, selectedDistro);
        handleClose();
      } finally {
        setLoading(false);
      }
    }
  };

  const handleClose = () => {
    if (!loading && !scanning) {
      onClose();
      setStep(1);
      setBaseId('claude-code');
      setEnvType('local');
      setLocalMethod('auto');
      setManualPath('');
      setInstallMethod('npm');
      setInstallerPath('');
      setInstallerCandidates([]);
      setToolCandidates([]);
      setSelectedToolCandidate(null);
      setValidationError(null);
      setSelectedDistro('');
      setScanResult(null);
    }
  };

  const handleNext = () => {
    // 验证第一步的选择
    if (envType === 'wsl' && !selectedDistro) {
      toast({
        variant: 'destructive',
        title: '请选择 WSL 发行版',
      });
      return;
    }
    setStep(2);
  };

  const handleBack = () => {
    setStep(1);
    setScanResult(null);
    setToolCandidates([]);
    setSelectedToolCandidate(null);
    setInstallerCandidates([]);
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && !loading && onClose()} modal>
      <DialogContent className="sm:max-w-[600px]" onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>添加工具实例</DialogTitle>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* 第一步：选择工具、环境类型、添加方式 */}
          {step === 1 && (
            <>
              {/* 选择工具 */}
              <div className="space-y-3">
                <Label className="text-base font-semibold">选择工具</Label>
                <div className="grid grid-cols-3 gap-3">
                  {TOOLS.map((tool) => (
                    <button
                      key={tool.id}
                      type="button"
                      onClick={() => setBaseId(tool.id)}
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
                      onClick={() => !env.disabled && setEnvType(env.id as 'local' | 'wsl' | 'ssh')}
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
                      <span className="text-xs text-muted-foreground text-center">
                        {env.description}
                      </span>
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
                        onClick={() => setLocalMethod(method.id as 'auto' | 'manual')}
                        className={cn(
                          'relative flex flex-col items-center justify-center py-2 px-3 rounded-lg border-2 transition-all hover:border-primary/50',
                          localMethod === method.id
                            ? 'border-primary bg-primary/5'
                            : 'border-border',
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

              {/* WSL 发行版选择 */}
              {envType === 'wsl' && (
                <div className="space-y-2">
                  <Label>选择WSL发行版</Label>
                  {loadingDistros ? (
                    <div className="rounded border p-3 bg-muted/50 text-sm text-center">
                      加载中...
                    </div>
                  ) : wslDistros.length === 0 ? (
                    <div className="rounded border p-3 bg-yellow-50 dark:bg-yellow-950/30">
                      <p className="text-sm text-yellow-800 dark:text-yellow-200">
                        未检测到WSL发行版，请先安装WSL
                      </p>
                    </div>
                  ) : (
                    <Select value={selectedDistro} onValueChange={setSelectedDistro}>
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
                  )}
                </div>
              )}
            </>
          )}

          {/* 第二步：配置详情/扫描 */}
          {step === 2 && envType === 'local' && (
            <>
              {localMethod === 'auto' && (
                <>
                  <Alert>
                    <InfoIcon className="h-4 w-4" />
                    <AlertDescription>
                      将自动扫描系统中已安装的 {toolNames[baseId]}，包括 npm、Homebrew 等安装方式
                    </AlertDescription>
                  </Alert>

                  <div className="space-y-2">
                    <Button
                      onClick={handleScan}
                      disabled={scanning}
                      className="w-full"
                      variant="outline"
                    >
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
                    {toolCandidates.length > 1 && (
                      <div className="space-y-2">
                        <Label>选择工具实例（共 {toolCandidates.length} 个）</Label>
                        <div className="space-y-2 max-h-60 overflow-y-auto">
                          {toolCandidates.map((candidate, index) => (
                            <button
                              key={index}
                              type="button"
                              onClick={() => {
                                setSelectedToolCandidate(candidate);
                                setScanResult({ installed: true, version: candidate.version });
                              }}
                              className={cn(
                                'w-full p-3 rounded-lg border-2 text-left transition-all hover:border-primary/50',
                                selectedToolCandidate === candidate
                                  ? 'border-primary bg-primary/5'
                                  : 'border-border',
                              )}
                            >
                              <div className="flex items-start justify-between">
                                <div className="space-y-1 flex-1">
                                  <div className="text-sm font-medium">{candidate.tool_path}</div>
                                  <div className="text-xs text-muted-foreground">
                                    版本：{candidate.version}
                                  </div>
                                  <div className="text-xs text-muted-foreground">
                                    安装器：{candidate.installer_path || '未检测到'}
                                  </div>
                                  <div className="text-xs text-muted-foreground">
                                    方法：{candidate.install_method}
                                  </div>
                                </div>
                                {selectedToolCandidate === candidate && (
                                  <CheckCircle2 className="h-5 w-5 text-primary flex-shrink-0" />
                                )}
                              </div>
                            </button>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* 单个候选时直接显示 */}
                    {toolCandidates.length === 1 && selectedToolCandidate && (
                      <Alert>
                        <InfoIcon className="h-4 w-4" />
                        <AlertDescription>
                          <div className="space-y-1">
                            <div>✓ 路径：{selectedToolCandidate.tool_path}</div>
                            <div>版本：{selectedToolCandidate.version}</div>
                            <div>安装器：{selectedToolCandidate.installer_path || '未检测到'}</div>
                          </div>
                        </AlertDescription>
                      </Alert>
                    )}
                  </div>
                </>
              )}

              {localMethod === 'manual' && (
                <>
                  <Alert>
                    <InfoIcon className="h-4 w-4" />
                    <AlertDescription className="space-y-2">
                      <p className="font-medium">常见安装路径：</p>
                      <ul className="list-disc list-inside text-xs space-y-1">
                        {getCommonPaths().map((path, index) => (
                          <li key={index} className="font-mono">
                            {path}
                          </li>
                        ))}
                      </ul>
                    </AlertDescription>
                  </Alert>

                  <div className="space-y-2">
                    <Label>可执行文件路径</Label>
                    <div className="flex gap-2">
                      <Input
                        value={manualPath}
                        onChange={(e) => {
                          setManualPath(e.target.value);
                          setValidationError(null);
                          setScanResult(null); // 清除扫描结果
                        }}
                        onBlur={() => {
                          if (manualPath) handleValidate(manualPath);
                        }}
                        placeholder="输入或浏览选择"
                        disabled={validating || loading || scanning}
                      />
                      <Button
                        onClick={handleBrowse}
                        variant="outline"
                        disabled={validating || loading || scanning}
                      >
                        浏览...
                      </Button>
                    </div>

                    {validating && (
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        验证中...
                      </div>
                    )}

                    {validationError && (
                      <Alert variant="destructive">
                        <AlertDescription>{validationError}</AlertDescription>
                      </Alert>
                    )}
                  </div>

                  {/* 安装器类型选择 */}
                  <div className="space-y-3">
                    <Label className="text-base font-semibold">安装器类型</Label>
                    <div className="grid grid-cols-4 gap-2">
                      {INSTALL_METHODS.map((method) => (
                        <button
                          key={method.id}
                          type="button"
                          onClick={() =>
                            setInstallMethod(method.id as 'npm' | 'brew' | 'official' | 'other')
                          }
                          className={cn(
                            'relative flex flex-col items-center justify-center py-2 px-2 rounded-lg border-2 transition-all hover:border-primary/50',
                            installMethod === method.id
                              ? 'border-primary bg-primary/5'
                              : 'border-border',
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

                  {/* 安装器路径（非 other 时显示） */}
                  {installMethod !== 'other' && (
                    <div className="space-y-2">
                      <Label>安装器路径（用于更新工具）</Label>

                      {/* 如果扫描到候选，显示选择列表 */}
                      {installerCandidates.length > 0 ? (
                        <>
                          <Select value={installerPath} onValueChange={setInstallerPath}>
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
                            已自动扫描到 {installerCandidates.length} 个安装器，可切换或手动输入
                          </p>
                        </>
                      ) : (
                        <>
                          <div className="flex gap-2">
                            <Input
                              value={installerPath}
                              onChange={(e) => setInstallerPath(e.target.value)}
                              placeholder={`如: ${navigator.platform.toLowerCase().includes('win') ? 'C:\\Program Files\\nodejs\\npm.cmd' : '/usr/local/bin/npm'}`}
                              disabled={loading || scanning}
                            />
                            <Button
                              onClick={handleBrowseInstaller}
                              variant="outline"
                              disabled={loading || scanning}
                            >
                              浏览...
                            </Button>
                          </div>
                          <p className="text-xs text-muted-foreground">
                            未检测到安装器，请手动选择或留空（无法快捷更新）
                          </p>
                        </>
                      )}
                    </div>
                  )}

                  {/* Other 类型警告 */}
                  {installMethod === 'other' && (
                    <Alert
                      variant="default"
                      className="border-yellow-500 bg-yellow-50 dark:bg-yellow-950/30"
                    >
                      <InfoIcon className="h-4 w-4 text-yellow-600" />
                      <AlertDescription className="text-yellow-800 dark:text-yellow-200">
                        <strong>「其他」类型不支持 APP 内快捷更新</strong>
                        <br />
                        您需要手动更新此工具
                      </AlertDescription>
                    </Alert>
                  )}

                  {/* 验证路径按钮 */}
                  <div>
                    <Button
                      onClick={handleScan}
                      disabled={scanning || !manualPath || !!validationError}
                      className="w-full"
                      variant="outline"
                    >
                      {scanning ? (
                        <>
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          验证中...
                        </>
                      ) : (
                        '验证路径'
                      )}
                    </Button>

                    {scanResult && (
                      <Alert variant={scanResult.installed ? 'default' : 'destructive'}>
                        <InfoIcon className="h-4 w-4" />
                        <AlertDescription>
                          {scanResult.installed ? (
                            <>
                              ✓ 验证成功：{toolNames[baseId]} v{scanResult.version}
                            </>
                          ) : (
                            <>验证失败</>
                          )}
                        </AlertDescription>
                      </Alert>
                    )}
                  </div>
                </>
              )}
            </>
          )}

          {/* WSL发行版选择 */}
          {step === 1 && envType === 'wsl' && (
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
                  <Select value={selectedDistro} onValueChange={setSelectedDistro}>
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
          )}

          {/* SSH配置表单（预留） */}
          {envType === 'ssh' && (
            <div className="rounded border p-3 bg-muted/50">
              <p className="text-sm text-muted-foreground">SSH功能将在后续版本提供</p>
            </div>
          )}
        </div>

        <DialogFooter>
          {step === 1 ? (
            <>
              <Button variant="outline" onClick={handleClose}>
                取消
              </Button>
              <Button onClick={handleNext}>下一步</Button>
            </>
          ) : (
            <>
              <Button variant="outline" onClick={handleBack} disabled={loading || scanning}>
                上一步
              </Button>
              <Button
                onClick={handleSubmit}
                disabled={
                  loading ||
                  scanning ||
                  envType === 'ssh' ||
                  (envType === 'local' && localMethod === 'auto' && !selectedToolCandidate) ||
                  (envType === 'local' &&
                    localMethod === 'manual' &&
                    (!scanResult || !scanResult.installed))
                }
              >
                {loading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    添加中...
                  </>
                ) : (
                  '添加'
                )}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
