use serde::Serialize;
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use url::Url;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub reachable: bool,
    pub status_code: u16,
    pub latency_ms: u64,
    pub endpoint: String,
    pub detail: String,
}

pub fn probe_gateway(base_url: &str, api_key: &str, auth_token: &str) -> ProbeResult {
    let normalized = if base_url.trim().is_empty() {
        "https://api.anthropic.com".to_string()
    } else {
        base_url.trim().to_string()
    };

    let parsed = match Url::parse(&normalized) {
        Ok(u) => u,
        Err(e) => {
            return ProbeResult {
                reachable: false,
                status_code: 0,
                latency_ms: 0,
                endpoint: normalized,
                detail: format!("URL 格式无效: {}", e),
            };
        }
    };

    let scheme = parsed.scheme();
    if scheme != "https" && scheme != "http" {
        return ProbeResult {
            reachable: false,
            status_code: 0,
            latency_ms: 0,
            endpoint: normalized,
            detail: format!("不支持的协议: {}", scheme),
        };
    }

    // Prefer system `curl` via stdin config (`-K -`) so secrets never appear in process argv
    if let Some(res) = probe_via_curl(&parsed, api_key, auth_token) {
        return res;
    }

    // Pure-Rust TCP socket fallback
    probe_via_tcp(&parsed)
}

fn sanitize_curl_config_value(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '\n' && *c != '\r' && *c != '"')
        .collect()
}

fn probe_via_curl(url: &Url, api_key: &str, auth_token: &str) -> Option<ProbeResult> {
    let mut probe_url = url.clone();
    let trimmed_path = probe_url.path().trim_end_matches('/');
    let models_path = format!("{}/v1/models", trimmed_path);
    probe_url.set_path(&models_path);

    let mut cfg = String::new();
    cfg.push_str(&format!(
        "url = \"{}\"\n",
        sanitize_curl_config_value(probe_url.as_str())
    ));
    cfg.push_str("silent\nshow-error\n");
    cfg.push_str("max-time = 6\nconnect-timeout = 4\n");
    cfg.push_str("write-out = \"%{http_code}\"\n");
    cfg.push_str("output = \"/dev/null\"\n");
    cfg.push_str("header = \"anthropic-version: 2023-06-01\"\n");

    if !auth_token.trim().is_empty() {
        cfg.push_str(&format!(
            "header = \"Authorization: Bearer {}\"\n",
            sanitize_curl_config_value(auth_token.trim())
        ));
    } else if !api_key.trim().is_empty() {
        cfg.push_str(&format!(
            "header = \"x-api-key: {}\"\n",
            sanitize_curl_config_value(api_key.trim())
        ));
    }

    let start = Instant::now();
    let mut child = Command::new("curl")
        .arg("-K")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(cfg.as_bytes());
    }

    let output = child.wait_with_output().ok()?;
    let latency_ms = start.elapsed().as_millis() as u64;
    let code_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let status_code = code_str.parse::<u16>().unwrap_or(0);

    if output.status.success() && status_code > 0 {
        let detail = match status_code {
            200..=299 => format!("HTTP {} · 鉴权与网关正常", status_code),
            401 | 403 => format!("HTTP {} · 网关在线 (TLS 正常)", status_code),
            404 | 405 => format!("HTTP {} · 端点可达 (TLS 正常)", status_code),
            429 => "HTTP 429 · 网关在线 (触发速率限制)".to_string(),
            _ => format!("HTTP {} · 服务端响应异常", status_code),
        };
        Some(ProbeResult {
            reachable: status_code < 500,
            status_code,
            latency_ms,
            endpoint: url.as_str().to_string(),
            detail,
        })
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Some(ProbeResult {
            reachable: false,
            status_code: 0,
            latency_ms,
            endpoint: url.as_str().to_string(),
            detail: if err_msg.is_empty() {
                "网关连接超时或无法解析主机".to_string()
            } else {
                err_msg
            },
        })
    }
}

fn probe_via_tcp(url: &Url) -> ProbeResult {
    let host = match url.host_str() {
        Some(h) => h,
        None => {
            return ProbeResult {
                reachable: false,
                status_code: 0,
                latency_ms: 0,
                endpoint: url.as_str().to_string(),
                detail: "缺少有效主机名".to_string(),
            }
        }
    };
    let port = url.port_or_known_default().unwrap_or(443);
    let addr_str = format!("{}:{}", host, port);
    let start = Instant::now();

    let mut addrs = match addr_str.to_socket_addrs() {
        Ok(a) => a,
        Err(e) => {
            return ProbeResult {
                reachable: false,
                status_code: 0,
                latency_ms: start.elapsed().as_millis() as u64,
                endpoint: url.as_str().to_string(),
                detail: format!("DNS 解析失败: {}", e),
            }
        }
    };

    if let Some(sock_addr) = addrs.next() {
        match TcpStream::connect_timeout(&sock_addr, Duration::from_secs(4)) {
            Ok(_) => ProbeResult {
                reachable: true,
                status_code: 200,
                latency_ms: start.elapsed().as_millis() as u64,
                endpoint: url.as_str().to_string(),
                detail: format!("TCP {}:{} 握手可达", host, port),
            },
            Err(e) => ProbeResult {
                reachable: false,
                status_code: 0,
                latency_ms: start.elapsed().as_millis() as u64,
                endpoint: url.as_str().to_string(),
                detail: format!("TCP 连接失败: {}", e),
            },
        }
    } else {
        ProbeResult {
            reachable: false,
            status_code: 0,
            latency_ms: start.elapsed().as_millis() as u64,
            endpoint: url.as_str().to_string(),
            detail: "未解析到可用地址".to_string(),
        }
    }
}
