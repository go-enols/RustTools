import { X, FileText, Image, Save } from 'lucide-react';
import { useState, useEffect, useCallback } from 'react';
import Editor from 'react-simple-code-editor';
import { highlight, languages } from 'prismjs';
import 'prismjs/components';
import 'prismjs/components/prism-python';
import 'prismjs/components/prism-javascript';
import 'prismjs/components/prism-typescript';
import 'prismjs/components/prism-json';
import 'prismjs/components/prism-yaml';
import 'prismjs/components/prism-bash';
import 'prismjs/themes/prism-tomorrow.css';
import { readTextFile, readBinaryFile, writeTextFile } from '../../../core/api';

interface FileViewerProps {
  fileName: string;
  filePath: string;
  onClose: () => void;
}

// Helper to check if file is an image
const isImageFile = (filename: string): boolean => {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  return ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'].includes(ext);
};

// Get MIME type for images
const getImageMimeType = (filename: string): string => {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  const mimeTypes: Record<string, string> = {
    jpg: 'jpeg',
    jpeg: 'jpeg',
    png: 'png',
    gif: 'gif',
    bmp: 'bmp',
    webp: 'webp',
  };
  return mimeTypes[ext] || 'jpeg';
};

// Get prism language for syntax highlighting
const getPrismLanguage = (filename: string): string => {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  const langMap: Record<string, string> = {
    py: 'python',
    python: 'python',
    js: 'javascript',
    jsx: 'javascript',
    ts: 'typescript',
    tsx: 'typescript',
    json: 'json',
    yaml: 'yaml',
    yml: 'yaml',
    sh: 'bash',
    bash: 'bash',
  };
  return langMap[ext] || 'javascript';
};

export default function FileViewer({ fileName, filePath, onClose }: FileViewerProps) {
  const [content, setContent] = useState<string | null>(null);
  const [originalContent, setOriginalContent] = useState<string | null>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isDirty, setIsDirty] = useState(false);
  const [saving, setSaving] = useState(false);

  const loadFile = useCallback(async () => {
    setLoading(true);
    setError(null);
    setContent(null);
    setOriginalContent(null);
    setImageUrl(null);
    setIsDirty(false);

    try {
      if (isImageFile(fileName)) {
        // For images, read as base64 and convert to data URL
        const result = await readBinaryFile(filePath);
        if (result.success && result.data) {
          const mimeType = getImageMimeType(fileName);
          const url = `data:image/${mimeType};base64,${result.data}`;
          setImageUrl(url);
        } else {
          setError(result.error || `无法加载文件: ${fileName}`);
        }
      } else {
        // For text files, read as string
        const result = await readTextFile(filePath);
        if (result.success && result.data !== undefined) {
          setContent(result.data);
          setOriginalContent(result.data);
        } else {
          setError(result.error || `无法加载文件: ${fileName}`);
        }
      }
    } catch (err) {
      console.error('Failed to load file:', err);
      setError(`无法加载文件: ${fileName}`);
    } finally {
      setLoading(false);
    }
  }, [fileName, filePath]);

  useEffect(() => {
    loadFile();
  }, [loadFile]);

  // Cleanup blob URL on unmount
  useEffect(() => {
    return () => {
      if (imageUrl) {
        URL.revokeObjectURL(imageUrl);
      }
    };
  }, [imageUrl]);

  const handleContentChange = (newContent: string) => {
    setContent(newContent);
    setIsDirty(newContent !== originalContent);
  };

  const handleSave = async () => {
    if (!content || !isDirty) return;

    setSaving(true);
    try {
      const result = await writeTextFile(filePath, content);
      if (result.success) {
        setOriginalContent(content);
        setIsDirty(false);
      } else {
        setError(result.error || '保存失败');
      }
    } catch (err) {
      console.error('Failed to save file:', err);
      setError('保存失败');
    } finally {
      setSaving(false);
    }
  };

  const highlightCode = (code: string): string => {
    const lang = getPrismLanguage(fileName);
    const grammar = languages[lang];
    if (grammar) {
      return highlight(code, grammar, lang);
    }
    return code;
  };

  return (
    <div className="file-viewer">
      <div className="file-viewer-header">
        <span className="file-viewer-title">
          {isImageFile(fileName) ? <Image size={16} /> : <FileText size={16} />}
          {fileName}
          {isDirty && <span className="file-viewer-dirty">*</span>}
        </span>
        <div className="file-viewer-actions">
          {!isImageFile(fileName) && isDirty && (
            <button
              className="file-viewer-save"
              onClick={handleSave}
              disabled={saving}
              title="保存"
            >
              <Save size={16} />
              {saving ? '保存中...' : '保存'}
            </button>
          )}
          <button className="file-viewer-close" onClick={onClose} title="关闭">
            <X size={18} />
          </button>
        </div>
      </div>
      <div className="file-viewer-content">
        {loading && <div className="file-viewer-loading">加载中...</div>}
        {error && <div className="file-viewer-error">{error}</div>}
        {imageUrl && !loading && (
          <div className="file-viewer-image-container">
            <img src={imageUrl} alt={fileName} className="file-viewer-image" />
          </div>
        )}
        {content !== null && !loading && !imageUrl && (
          <div className="file-viewer-code-container">
            <Editor
              value={content}
              onValueChange={handleContentChange}
              highlight={highlightCode}
              padding={16}
              className="file-viewer-editor"
              style={{
                fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
                fontSize: '13px',
                lineHeight: 1.6,
                minHeight: '100%',
              }}
            />
          </div>
        )}
      </div>
    </div>
  );
}
