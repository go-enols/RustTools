//! 网络搜索工具
//!
//! 让 AI 能够通过搜索引擎获取最新信息。
//! 当前使用 DuckDuckGo Lite（无需 API Key）。

use super::tool::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// 网络搜索工具
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "通过网络搜索引擎获取最新信息。当需要验证事实、查询最新技术文档、\
         或获取知识截止日期之后的信息时调用此工具。"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "搜索关键词"
                },
                "count": {
                    "type": "integer",
                    "description": "返回结果数量，默认为 5，最大 10"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("缺少 query 参数".to_string()))?;

        let count = params
            .get("count")
            .and_then(|v| v.as_u64())
            .map(|c| (c as usize).clamp(1, 10))
            .unwrap_or(5);

        match search_duckduckgo(query, count).await {
            Ok(results) => Ok(ToolResult::ok_json(results)),
            Err(e) => Ok(ToolResult::err(format!("搜索失败: {}", e))),
        }
    }
}

/// 使用 DuckDuckGo Lite 进行搜索
async fn search_duckduckgo(query: &str, count: usize) -> Result<String, ToolError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; RustTools-Agent/1.0)")
        .build()
        .map_err(|e| ToolError::Other(format!("创建 HTTP 客户端失败: {}", e)))?;

    let url = format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        encode_url_component(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ToolError::Other(format!("请求失败: {}", e)))?;

    let html = response
        .text()
        .await
        .map_err(|e| ToolError::Other(format!("读取响应失败: {}", e)))?;

    let results = parse_duckduckgo_results(&html, count)?;

    let output = serde_json::json!({
        "query": query,
        "engine": "DuckDuckGo Lite",
        "results": results,
    });

    serde_json::to_string_pretty(&output)
        .map_err(|e| ToolError::JsonParse(e.to_string()))
}

/// 简易 URL 编码
fn encode_url_component(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    result
}

/// 解析 DuckDuckGo Lite HTML 结果
fn parse_duckduckgo_results(html: &str, max_count: usize) -> Result<Vec<Value>, ToolError> {
    let mut results = Vec::new();

    // DuckDuckGo Lite 结果格式相对简单
    // 每个结果在一个 .result-link 或类似结构中
    // 这里使用简单的字符串匹配来提取

    // 提取标题和链接
    let link_pattern = "<a rel=\"nofollow\" href=\"";
    let mut pos = 0;

    while let Some(link_start) = html[pos..].find(link_pattern) {
        let link_start = pos + link_start + link_pattern.len();
        if let Some(link_end) = html[link_start..].find("\"") {
            let url = &html[link_start..link_start + link_end];

            // 尝试提取标题（在链接后的 > 和 < 之间）
            let after_link = link_start + link_end;
            if let Some(title_start) = html[after_link..].find(">") {
                let title_start = after_link + title_start + 1;
                if let Some(title_end) = html[title_start..].find("</a>") {
                    let title = html[title_start..title_start + title_end]
                        .trim()
                        .to_string();

                    // 清理 HTML 实体
                    let title = title
                        .replace("&amp;", "&")
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&quot;", "\"");

                    if !title.is_empty() && !url.starts_with("/") {
                        results.push(serde_json::json!({
                            "title": title,
                            "url": url,
                        }));
                    }

                    if results.len() >= max_count {
                        break;
                    }
                }
            }
        }
        pos = link_start;
    }

    Ok(results)
}
