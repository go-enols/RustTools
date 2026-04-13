import { useState, useEffect } from 'react';
import { CheckCircle, XCircle, AlertCircle, Download, Loader2, Terminal } from 'lucide-react';
import {
  checkPythonEnv,
  getCachedPythonEnv,
  getInstallInstructions,
  installPythonDeps,
  type PythonEnvInfo,
  type InstallInstructions,
} from '../../../core/api/training';

interface PythonEnvCheckProps {
  onClose?: () => void;
}

export default function PythonEnvCheck({ onClose }: PythonEnvCheckProps) {
  const [envInfo, setEnvInfo] = useState<PythonEnvInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);  // 区分首次加载和手动刷新
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState('');
  const [installError, setInstallError] = useState<string | null>(null);
  const [showInstallModal, setShowInstallModal] = useState(false);
  const [useMirror, setUseMirror] = useState(true);
  const [showManualInstall, setShowManualInstall] = useState(false);
  const [cpuOnly, setCpuOnly] = useState(false);
  const [showDowngradeModal, setShowDowngradeModal] = useState(false);
  const [instructions, setInstructions] = useState<InstallInstructions | null>(null);
  const [fromCache, setFromCache] = useState(false);  // 标记数据来源

  useEffect(() => {
    loadInstructions();
    // 首次加载：优先使用缓存
    loadEnvironmentInfo();
  }, []);

  /**
   * 加载环境信息 - 优先使用缓存，避免重复检测
   */
  const loadEnvironmentInfo = async () => {
    setLoading(true);
    setFromCache(false);
    
    try {
      // 尝试获取缓存的环境信息
      const cached = await getCachedPythonEnv();
      
      if (cached.success && cached.data) {
        // 有缓存，直接使用
        setEnvInfo(cached.data);
        setFromCache(true);
        setLoading(false);
        return;
      }
      
      // 没有缓存，执行检测
      await checkEnvironment();
    } catch (error) {
      console.error('[PythonEnvCheck] Failed to load environment info:', error);
      // 检测失败时显示空状态
      setEnvInfo({
        python_exists: false,
        python_version: null,
        torch_exists: false,
        torch_version: null,
        torchaudio_exists: false,
        cuda_available: false,
        cuda_version: null,
        ultralytics_exists: false,
        ultralytics_version: null,
        yolo_command_exists: false,
      });
      setLoading(false);
    }
  };

  /**
   * 手动重新检测环境
   */
  const handleRefresh = async () => {
    setIsRefreshing(true);
    await checkEnvironment(true);  // 强制刷新
    setIsRefreshing(false);
  };

  /**
   * 执行环境检测
   * @param forceRefresh - 是否强制刷新（绕过缓存）
   */
  const checkEnvironment = async (forceRefresh: boolean = false) => {
    setLoading(true);
    
    try {
      const result = await checkPythonEnv(forceRefresh);

      if (result.success && result.data) {
        setEnvInfo(result.data);
        setFromCache(false);  // 新检测的数据不是缓存
      } else {
        setEnvInfo({
          python_exists: false,
          python_version: null,
          torch_exists: false,
          torch_version: null,
          torchaudio_exists: false,
          cuda_available: false,
          cuda_version: null,
          ultralytics_exists: false,
          ultralytics_version: null,
          yolo_command_exists: false,
        });
      }
    } catch (error) {
      console.error('[PythonEnvCheck] Environment check failed:', error);
      setEnvInfo({
        python_exists: false,
        python_version: null,
        torch_exists: false,
        torch_version: null,
        torchaudio_exists: false,
        cuda_available: false,
        cuda_version: null,
        ultralytics_exists: false,
        ultralytics_version: null,
        yolo_command_exists: false,
      });
    } finally {
      setLoading(false);
    }
  };

  const loadInstructions = async () => {
    const result = await getInstallInstructions();
    setInstructions(result);
  };

  /**
   * 处理安装依赖
   */
  const handleInstall = async () => {
    setInstalling(true);
    setInstallError(null);
    setInstallProgress('正在安装 Python 依赖...');

    const result = await installPythonDeps(useMirror, cpuOnly);

    if (result.success) {
      setInstallProgress('安装成功！');
      setTimeout(async () => {
        setShowInstallModal(false);
        // 安装完成后强制刷新检测
        await checkEnvironment(true);
      }, 1500);
    } else {
      setInstallError(result.error || '安装失败');
    }
    setInstalling(false);
  };

  const isEnvReady = envInfo?.python_exists && envInfo?.torch_exists && envInfo?.ultralytics_exists;

  if (loading && !envInfo) {
    return (
      <div style={{
        padding: 'var(--spacing-xl)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 'var(--spacing-md)'
      }}>
        <Loader2 size={20} className="animate-spin" style={{ animation: 'spin 1s linear infinite' }} />
        <span>正在检测 Python 环境...</span>
      </div>
    );
  }

  return (
    <div style={{ padding: 'var(--spacing-xl)' }}>
      <style>{`
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
        .env-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 16px;
          background: var(--bg-surface);
          border-radius: 8px;
          margin-bottom: 8px;
        }
        .env-status {
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .env-version {
          font-size: 12px;
          color: var(--text-tertiary);
          font-family: monospace;
        }
        .install-modal {
          position: fixed;
          inset: 0;
          background: rgba(0, 0, 0, 0.7);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 2000;
        }
        .install-content {
          background: var(--bg-elevated);
          border: 1px solid var(--border-default);
          border-radius: 12px;
          min-width: 450px;
          max-width: 500px;
          box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
        }
        .install-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 16px 20px;
          border-bottom: 1px solid var(--border-default);
        }
        .install-body {
          padding: 20px;
        }
        .install-error {
          background: rgba(255, 77, 79, 0.1);
          border: 1px solid var(--color-danger, #ff4d4f);
          border-radius: 8px;
          padding: 12px;
          margin-bottom: 16px;
          font-size: 13px;
          color: var(--color-danger, #ff4d4f);
        }
        .install-progress {
          display: flex;
          align-items: center;
          gap: 12px;
          color: var(--accent-primary);
          font-size: 14px;
        }
        .mirror-option {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-bottom: 16px;
          font-size: 13px;
        }
        .mirror-option input[type="checkbox"] {
          width: 16px;
          height: 16px;
        }
        .command-block {
          background: var(--bg-input);
          border: 1px solid var(--border-default);
          border-radius: 6px;
          padding: 12px;
          margin-bottom: 8px;
          font-family: monospace;
          font-size: 12px;
          overflow-x: auto;
        }
        .command-block code {
          color: var(--accent-primary);
        }
        .manual-section {
          margin-top: 16px;
          padding-top: 16px;
          border-top: 1px solid var(--border-default);
        }
        .manual-title {
          font-size: 14px;
          font-weight: 500;
          margin-bottom: 12px;
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .btn {
          padding: 8px 16px;
          font-size: 13px;
          font-weight: 500;
          border-radius: 6px;
          cursor: pointer;
          transition: all 0.15s;
          border: none;
        }
        .btn-primary {
          background: var(--accent-primary);
          color: var(--text-primary);
        }
        .btn-primary:hover:not(:disabled) {
          background: var(--accent-hover);
        }
        .btn-primary:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .btn-secondary {
          background: var(--bg-hover);
          color: var(--text-primary);
          border: 1px solid var(--border-default);
        }
        .btn-secondary:hover {
          background: var(--bg-active);
        }
      `}</style>

      {/* Environment Status */}
      <div style={{ marginBottom: 'var(--spacing-xl)' }}>
        <h3 style={{ fontSize: 14, color: 'var(--text-secondary)', marginBottom: 'var(--spacing-md)', display: 'flex', alignItems: 'center', gap: '8px' }}>
          <Terminal size={16} />
          Python 环境状态
        </h3>

        <div className="env-item">
          <div className="env-status">
            {envInfo?.python_exists ? (
              <CheckCircle size={18} color="var(--status-success, #52c41a)" />
            ) : (
              <XCircle size={18} color="var(--color-danger, #ff4d4f)" />
            )}
            <span>Python</span>
            {envInfo?.python_version && (
              <span className="env-version">{envInfo.python_version}</span>
            )}
          </div>
        </div>

        {/* Python Not Installed Alert */}
        {!envInfo?.python_exists && (
          <div style={{
            background: 'rgba(255, 77, 79, 0.1)',
            border: '1px solid rgba(255, 77, 79, 0.3)',
            borderRadius: 8,
            padding: '12px 16px',
            marginTop: 8,
            display: 'flex',
            alignItems: 'flex-start',
            gap: 12
          }}>
            <AlertCircle size={18} color="var(--color-danger, #ff4d4f)" style={{ flexShrink: 0, marginTop: 2 }} />
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 13, color: 'var(--text-primary)', marginBottom: 4, fontWeight: 500 }}>
                请安装 Python 3.11.8
              </div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                本应用需要 Python 3.11.8 版本。请从
                <a
                  href="https://www.python.org/ftp/python/3.11.8/python-3.11.8-amd64.exe"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: 'var(--accent-primary)', marginLeft: 4 }}
                >
                  Python 官网
                </a>
                下载安装。安装时请勾选「Add Python to PATH」。
              </div>
            </div>
          </div>
        )}

        <div className="env-item">
          <div className="env-status">
            {envInfo?.torch_exists ? (
              <CheckCircle size={18} color="var(--status-success, #52c41a)" />
            ) : (
              <XCircle size={18} color="var(--color-danger, #ff4d4f)" />
            )}
            <span>PyTorch</span>
            {envInfo?.torch_version && (
              <span className="env-version">{envInfo.torch_version}</span>
            )}
          </div>
        </div>

        <div className="env-item">
          <div className="env-status">
            {envInfo?.torchaudio_exists ? (
              <CheckCircle size={18} color="var(--status-success, #52c41a)" />
            ) : (
              <AlertCircle size={18} color="var(--status-warning, #faad14)" />
            )}
            <span>torchaudio</span>
            {!envInfo?.torchaudio_exists && (
              <span className="env-version">可选</span>
            )}
          </div>
        </div>

        <div className="env-item">
          <div className="env-status">
            {envInfo?.cuda_available ? (
              <CheckCircle size={18} color="var(--status-success, #52c41a)" />
            ) : (
              <AlertCircle size={18} color="var(--status-warning, #faad14)" />
            )}
            <span>CUDA</span>
            {envInfo?.cuda_available && envInfo?.cuda_version ? (
              <span className="env-version">{envInfo.cuda_version}</span>
            ) : (
              <span className="env-version">不可用 (将使用 CPU)</span>
            )}
          </div>
        </div>

        {/* CUDA Compatibility Warning */}
        {envInfo?.torch_exists && !envInfo?.cuda_available && (
          <div style={{
            background: 'rgba(250, 173, 20, 0.1)',
            border: '1px solid rgba(250, 173, 20, 0.3)',
            borderRadius: 8,
            padding: '12px 16px',
            marginTop: 8,
            display: 'flex',
            alignItems: 'flex-start',
            gap: 12
          }}>
            <AlertCircle size={18} color="var(--status-warning, #faad14)" style={{ flexShrink: 0, marginTop: 2 }} />
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 13, color: 'var(--text-primary)', marginBottom: 4, fontWeight: 500 }}>
                PyTorch 与 CUDA 可能存在兼容性问题
              </div>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                检测到 PyTorch 已安装但无法使用 CUDA。这通常是因为 PyTorch 的 CUDA 版本与您的系统不匹配。
                点击「查看解决方案」获取帮助。
              </div>
            </div>
            <button
              className="btn btn-secondary"
              onClick={() => setShowDowngradeModal(true)}
              style={{ flexShrink: 0, fontSize: 12, padding: '6px 12px' }}
            >
              查看解决方案
            </button>
          </div>
        )}

        <div className="env-item">
          <div className="env-status">
            {envInfo?.ultralytics_exists ? (
              <CheckCircle size={18} color="var(--status-success, #52c41a)" />
            ) : (
              <XCircle size={18} color="var(--color-danger, #ff4d4f)" />
            )}
            <span>Ultralytics</span>
            {envInfo?.ultralytics_version && (
              <span className="env-version">{envInfo.ultralytics_version}</span>
            )}
          </div>
        </div>

        <div className="env-item">
          <div className="env-status">
            {envInfo?.yolo_command_exists ? (
              <CheckCircle size={18} color="var(--status-success, #52c41a)" />
            ) : (
              <AlertCircle size={18} color="var(--status-warning, #faad14)" />
            )}
            <span>YOLO CLI</span>
            {envInfo?.yolo_command_exists && (
              <span className="env-version">可用</span>
            )}
            {!envInfo?.yolo_command_exists && (
              <span className="env-version">安装 ultralytics 后可用</span>
            )}
          </div>
        </div>
      </div>

      {/* Action Buttons */}
      <div style={{ display: 'flex', gap: 'var(--spacing-md)', alignItems: 'center' }}>
        {!isEnvReady && (
          <button
            className="btn btn-primary"
            onClick={() => setShowInstallModal(true)}
            style={{ display: 'flex', alignItems: 'center', gap: '8px' }}
          >
            <Download size={16} />
            立即安装
          </button>
        )}
        <button 
          className="btn btn-secondary" 
          onClick={handleRefresh}
          disabled={isRefreshing}
          style={{ display: 'flex', alignItems: 'center', gap: '8px' }}
        >
          {isRefreshing ? (
            <>
              <Loader2 size={16} style={{ animation: 'spin 1s linear infinite' }} />
              检测中...
            </>
          ) : (
            '重新检测'
          )}
        </button>
        {fromCache && !isRefreshing && (
          <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
            (已缓存)
          </span>
        )}
        {onClose && (
          <button className="btn btn-secondary" onClick={onClose}>
            关闭
          </button>
        )}
      </div>

      {/* Status Message */}
      {isEnvReady && (
        <div style={{
          marginTop: 'var(--spacing-lg)',
          padding: '12px 16px',
          background: 'rgba(82, 196, 26, 0.1)',
          border: '1px solid var(--status-success, #52c41a)',
          borderRadius: '8px',
          fontSize: 13,
          color: 'var(--status-success, #52c41a)',
          display: 'flex',
          alignItems: 'center',
          gap: '8px'
        }}>
          <CheckCircle size={16} />
          环境就绪，可以开始训练！
        </div>
      )}

      {/* Install Modal */}
      {showInstallModal && (
        <div className="install-modal" onClick={() => !installing && setShowInstallModal(false)}>
          <div className="install-content" onClick={(e) => e.stopPropagation()}>
            <div className="install-header">
              <h3 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>
                安装 Python 依赖
              </h3>
              {!installing && (
                <button
                  onClick={() => setShowInstallModal(false)}
                  style={{
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    padding: 4,
                    color: 'var(--text-secondary)'
                  }}
                >
                  ✕
                </button>
              )}
            </div>
            <div className="install-body">
              {installError && (
                <div className="install-error">
                  <strong>安装失败</strong>
                  <p style={{ margin: '8px 0 0 0' }}>{installError}</p>
                </div>
              )}

              {installing ? (
                <div className="install-progress">
                  <Loader2 size={20} style={{ animation: 'spin 1s linear infinite' }} />
                  <span>{installProgress || '正在安装...'}</span>
                </div>
              ) : (
                <>
                  <div className="mirror-option">
                    <input
                      type="checkbox"
                      id="useMirror"
                      checked={useMirror}
                      onChange={(e) => setUseMirror(e.target.checked)}
                    />
                    <label htmlFor="useMirror">
                      使用国内镜像（清华大学）加速下载
                    </label>
                  </div>

                  <div className="mirror-option">
                    <input
                      type="checkbox"
                      id="cpuOnly"
                      checked={cpuOnly}
                      onChange={(e) => setCpuOnly(e.target.checked)}
                    />
                    <label htmlFor="cpuOnly">
                      使用 CPU 版本（训练速度较慢，但可避免 CUDA 兼容性问题）
                    </label>
                  </div>

                  <p style={{ fontSize: 13, color: 'var(--text-secondary)', marginBottom: 16 }}>
                    将安装: PyTorch {cpuOnly ? '(CPU)' : '(GPU)'}, Ultralytics, onnxruntime
                  </p>

                  <div style={{ display: 'flex', gap: 'var(--spacing-md)' }}>
                    <button
                      className="btn btn-primary"
                      onClick={handleInstall}
                      style={{ flex: 1 }}
                    >
                      开始安装
                    </button>
                    <button
                      className="btn btn-secondary"
                      onClick={() => setShowManualInstall(!showManualInstall)}
                    >
                      手动安装
                    </button>
                  </div>

                  {showManualInstall && instructions && (
                    <div className="manual-section">
                      <div className="manual-title">
                        <AlertCircle size={16} />
                        手动安装指南
                      </div>

                      <p style={{ fontSize: 12, color: 'var(--text-tertiary)', marginBottom: 12 }}>
                        如果自动安装失败，可以复制以下命令手动安装：
                      </p>

                      <div style={{ marginBottom: 12 }}>
                        <strong style={{ fontSize: 12 }}>1. 升级 pip</strong>
                        {instructions.pip_install.map((cmd, i) => (
                          <div key={i} className="command-block">
                            <code>{cmd}</code>
                          </div>
                        ))}
                      </div>

                      <div style={{ marginBottom: 12 }}>
                        <strong style={{ fontSize: 12 }}>2. 安装 PyTorch（GPU版本）</strong>
                        {instructions.torch_install.map((cmd, i) => (
                          <div key={i} className="command-block">
                            <code>{cmd}</code>
                          </div>
                        ))}
                      </div>

                      <div style={{ marginBottom: 12 }}>
                        <strong style={{ fontSize: 12, color: 'var(--status-success-text)' }}>2. 安装 PyTorch（CPU版本 - 推荐）</strong>
                        <p style={{ fontSize: 11, color: 'var(--text-tertiary)', margin: '4px 0 8px 0' }}>
                          如遇 CUDA 兼容问题，推荐安装此版本
                        </p>
                        {instructions.torch_cpu_install.map((cmd, i) => (
                          <div key={i} className="command-block">
                            <code>{cmd}</code>
                          </div>
                        ))}
                      </div>

                      <div style={{ marginBottom: 12 }}>
                        <strong style={{ fontSize: 12 }}>3. 安装 Ultralytics</strong>
                        {instructions.ultralytics_install.map((cmd, i) => (
                          <div key={i} className="command-block">
                            <code>{cmd}</code>
                          </div>
                        ))}
                      </div>

                      <div>
                        <strong style={{ fontSize: 12 }}>官方网站</strong>
                        {instructions.manual_download.map((url, i) => (
                          <div key={i} className="command-block">
                            <code>{url}</code>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Downgrade Modal */}
      {showDowngradeModal && (
        <div className="install-modal" onClick={() => setShowDowngradeModal(false)}>
          <div className="install-content" onClick={(e) => e.stopPropagation()}>
            <div className="install-header">
              <div style={{ fontSize: 16, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 8 }}>
                <AlertCircle size={18} color="var(--status-warning, #faad14)" />
                CUDA 兼容性问题处理
              </div>
              <button
                onClick={() => setShowDowngradeModal(false)}
                style={{
                  background: 'none',
                  border: 'none',
                  cursor: 'pointer',
                  padding: 4,
                  color: 'var(--text-tertiary)',
                  fontSize: 18
                }}
              >
                ×
              </button>
            </div>
            <div className="install-body">
              <div style={{
                background: 'rgba(250, 173, 20, 0.1)',
                border: '1px solid rgba(250, 173, 20, 0.3)',
                borderRadius: 8,
                padding: '12px 16px',
                marginBottom: 20
              }}>
                <div style={{ fontSize: 13, fontWeight: 500, marginBottom: 8 }}>
                  问题原因
                </div>
                <div style={{ fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.6 }}>
                  您的 PyTorch 版本可能与系统 CUDA 版本不匹配。常见原因：
                  <ul style={{ margin: '8px 0 0 0', paddingLeft: 16 }}>
                    <li>PyTorch 安装了 CUDA 12.x 但系统只有 CUDA 11.x</li>
                    <li>PyTorch 安装了 CUDA 11.x 但系统是 CUDA 12.x</li>
                    <li>GPU 驱动版本过低，不支持当前 CUDA 版本</li>
                  </ul>
                </div>
              </div>

              <div style={{ marginBottom: 20 }}>
                <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
                  解决方案
                </div>

                <div style={{
                  border: '1px solid var(--border-default)',
                  borderRadius: 8,
                  padding: 16,
                  marginBottom: 12
                }}>
                  <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--status-success-text)', marginBottom: 8 }}>
                    方案一：使用 CPU 版本训练
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 12 }}>
                    本应用支持 CPU 模式进行训练，训练速度会变慢但可以正常运行。
                    您可以在安装弹框中勾选「使用 CPU 版本」。
                  </div>
                </div>

                <div style={{
                  border: '1px solid var(--border-default)',
                  borderRadius: 8,
                  padding: 16
                }}>
                  <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--accent-primary)', marginBottom: 8 }}>
                    方案二：手动选择匹配的 CUDA 版本
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 12 }}>
                    如果您知道系统的 CUDA 版本，可以手动安装对应版本：
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 8 }}>
                    1. 先卸载当前版本：
                  </div>
                  <div className="command-block" style={{ marginBottom: 12 }}>
                    <code>pip uninstall torch torchvision -y</code>
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 8 }}>
                    2. 安装匹配 CUDA 11.8 的版本：
                  </div>
                  <div className="command-block" style={{ marginBottom: 4 }}>
                    <code>pip install torch torchvision --index-url https://download.pytorch.org/whl/cu118</code>
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 8, marginTop: 12 }}>
                    3. 安装匹配 CUDA 12.1 的版本：
                  </div>
                  <div className="command-block" style={{ marginBottom: 4 }}>
                    <code>pip install torch torchvision --index-url https://download.pytorch.org/whl/cu121</code>
                  </div>
                </div>
              </div>

              <div style={{ display: 'flex', gap: 'var(--spacing-md)', justifyContent: 'flex-end' }}>
                <button
                  className="btn btn-secondary"
                  onClick={() => setShowDowngradeModal(false)}
                >
                  关闭
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
