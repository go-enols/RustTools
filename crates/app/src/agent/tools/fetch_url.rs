//! 网页内容获取工具 (FetchURL)
//!
//! 获取网页内容并提取主要文本，支持：
//! - 直接 HTTP GET 获取页面
//! - 去除 HTML 标签提取纯文本
//! - 自动处理编码
//! - 内容长度限制和截断

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 网页内容获取工具
///
/// 参数:
/// - `url`: 目标 URL（必填）
/// - `max_length`: 最大返回字符数（可选，默认 10000）
pub struct FetchUrlTool;

impl FetchUrlTool {
    pub fn new() -> Self {
        Self
    }

    /// 简单 HTML 到文本的转换
    /// 移除 script/style 标签，提取文本内容
    fn html_to_text(html: &str) -> String {
        let mut text = html.to_string();

        // 移除 script 和 style 标签及其内容
        for tag in &["script", "style", "nav", "footer", "header", "aside", "noscript"] {
            let open = format!("<{}", tag);
            let close = format!("</{}>", tag);
            
            loop {
                if let Some(start) = text.to_lowercase().find(&open) {
                    if let Some(end) = text[start..].to_lowercase().find(&close) {
                        let end_pos = start + end + close.len();
                        text.replace_range(start..end_pos, "");
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        // 移除 HTML 标签
        let mut result = String::with_capacity(text.len());
        let mut in_tag = false;
        let mut prev_char = ' ';
        
        for ch in text.chars() {
            if ch == '<' {
                in_tag = true;
            } else if ch == '>' {
                in_tag = false;
            } else if !in_tag {
                // 合并多个空白字符
                if ch.is_whitespace() {
                    if !prev_char.is_whitespace() {
                        result.push(' ');
                        prev_char = ' ';
                    }
                } else {
                    result.push(ch);
                    prev_char = ch;
                }
            }
        }

        // 解码常见 HTML 实体
        let result = result
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ");

        result.trim().to_string()
    }

    /// 尝试提取主要内容（通过一些启发式规则）
    fn extract_main_content(html: &str) -> String {
        let text = Self::html_to_text(html);
        
        // 如果文本较短，直接返回
        if text.len() < 5000 {
            return text;
        }

        // 尝试提取 article/main/div 等主要内容区域
        // 简单启发式：找最长的连续文本段落
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut longest_start = 0;
        let mut longest_len = 0;
        let mut current_start = 0;
        let mut current_len = 0;

        for (i, para) in paragraphs.iter().enumerate() {
            let para_len = para.len();
            if para_len > 50 {
                current_len += para_len;
            } else {
                if current_len > longest_len {
                    longest_len = current_len;
                    longest_start = current_start;
                }
                current_start = i + 1;
                current_len = 0;
            }
        }

        if current_len > longest_len {
            longest_len = current_len;
            longest_start = current_start;
        }

        // 如果最长的内容段太短，返回全部文本
        if longest_len < text.len() / 10 {
            return text;
        }

        paragraphs[longest_start..]
            .iter()
            .take_while(|p| p.len() > 20 || p.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[async_trait]
impl Tool for FetchUrlTool {
    fn name(&self) -> &str {
        "fetch_url"
    }

    fn description(&self) -> &str {
        "获取网页内容并提取主要文本。用于读取文档、博客、API 文档等网页内容。\
         支持自动去除 HTML 标签、提取正文。\
         注意：不要用于获取图片或视频，请使用 fs_read 读取本地媒体文件。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "要获取的网页 URL"
                },
                "max_length": {
                    "type": "integer",
                    "description": "最大返回字符数，默认 10000",
                    "minimum": 100,
                    "maximum": 50000,
                    "default": 10000
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParameters("缺少 url 参数".to_string()))?;
        
        let max_length = params["max_length"].as_u64()
            .map(|c| (c as usize).clamp(100, 50000))
            .unwrap_or(10000);

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidParameters(
                "URL 必须以 http:// 或 https:// 开头".to_string()
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (compatible; RustTools-Agent/1.0)")
            .build()
            .map_err(|e| ToolError::Other(format!("创建 HTTP 客户端失败: {}", e)))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::Other(format!("请求失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult::err(format!("HTTP 错误: {} {}", status.as_u16(), status)));
        }

        let html = response
            .text()
            .await
            .map_err(|e| ToolError::Other(format!("读取响应失败: {}", e)))?;

        let content = Self::extract_main_content(&html);
        let content_len = content.len();
        
        let (result, truncated) = if content_len > max_length {
            let mut cut = content[..max_length].to_string();
            // 尝试在句子边界截断
            if let Some(pos) = cut.rfind(|c| c == '.' || c == '。' || c == '\n') {
                if pos > max_length * 3 / 4 {
                    cut.truncate(pos + 1);
                }
            }
            (cut, true)
        } else {
            (content, false)
        };

        let mut output = format!("URL: {}\n", url);
        if truncated {
            output.push_str(&format!("\n[内容已截断，原始长度 {} 字符，显示前 {} 字符]\n\n", 
                content_len, result.len()));
        }
        output.push_str(&result);

        Ok(ToolResult::ok(output))
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_text() {
        let html = r#"<html><body><p>Hello <b>world</b>!</p><script>alert('x')</script></body></html>"#;
        let text = FetchUrlTool::html_to_text(html);
        assert!(text.contains("Hello world!"));
        assert!(!text.contains("script"));
        assert!(!text.contains("alert"));
    }

    #[test]
    fn test_html_to_text_entities() {
        let html = r#"<p>Foo &amp; Bar &lt; Baz &gt; Qux &quot;test&quot;</p>"#;
        let text = FetchUrlTool::html_to_text(html);
        assert!(text.contains("Foo & Bar < Baz > Qux \"test\""));
    }

    #[test]
    fn test_extract_main_content() {
        let html = r#"
            <html><body>
            <header>Short nav</header>
            <main>
            <p>This is a long paragraph with lots of content about something important.</p>
            <p>Another paragraph with more details and information.</p>
            <p>Third paragraph continuing the main content section.</p>
            </main>
            <footer>Short footer</footer>
            </body></html>
        "#;
        let text = FetchUrlTool::extract_main_content(html);
        assert!(text.contains("long paragraph"));
        assert!(!text.contains("alert"));
    }
}
