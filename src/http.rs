//! HTTP 协议层（成员A负责，完整实现）
//!
//! 纯 std 极简 HTTP：GET + POST 表单 + Cookie，每连接一个请求后关闭。
//! 从上一课程项目（kvstore）的 HTTP 层移植，并新增 Cookie 支持用于登录会话。

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

/// 解析后的 HTTP 请求
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// "GET" / "POST"
    pub method: String,
    /// 路径（不含查询串），如 "/login" "/dashboard"
    pub path: String,
    /// URL 查询参数（已解码）
    pub query: Vec<(String, String)>,
    /// POST 表单字段（已解码）
    pub body: Vec<(String, String)>,
    /// Cookie 头原始内容（无则为空串）
    pub cookie: String,
}

impl HttpRequest {
    /// 取查询参数或表单字段值（先查 query 再查 body）
    pub fn param(&self, name: &str) -> Option<String> {
        self.query
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .or_else(|| {
                self.body
                    .iter()
                    .find(|(k, _)| k == name)
                    .map(|(_, v)| v.clone())
            })
    }
}

/// 从带缓冲读取器解析一个 HTTP 请求
pub fn parse_request(reader: &mut impl BufRead) -> Option<HttpRequest> {
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let request_line = request_line.trim_end();
    if request_line.is_empty() {
        return None;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;

    let (path, query_string) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), Some(q)),
        None => (target.to_string(), None),
    };
    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(q) = query_string {
        for pair in q.split('&') {
            if !pair.is_empty() {
                match pair.split_once('=') {
                    Some((k, v)) => query.push((url_decode(k), url_decode(v))),
                    None => query.push((url_decode(pair), String::new())),
                }
            }
        }
    }

    // 头部（找 Content-Length 与 Cookie）
    let mut content_length: usize = 0;
    let mut cookie = String::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let name = name.trim();
            let value = value.trim();
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("cookie") {
                cookie = value.to_string();
            }
        }
    }

    let mut body = String::new();
    if content_length > 0 {
        let mut buf = vec![0u8; content_length];
        if reader.read_exact(&mut buf).is_err() {
            return None;
        }
        body = String::from_utf8_lossy(&buf).to_string();
    }

    Some(HttpRequest {
        method,
        path,
        query,
        body: parse_form(&body),
        cookie,
    })
}

/// URL 解码（%XX 按字节还原，+ 转空格）
pub fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(v) => {
                    out.push(v);
                    i += 2;
                }
                Err(_) => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// 解析表单体 key=value&...
pub fn parse_form(body: &str) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();
    for pair in body.split('&') {
        if !pair.is_empty() {
            match pair.split_once('=') {
                Some((k, v)) => fields.push((url_decode(k), url_decode(v))),
                None => fields.push((url_decode(pair), String::new())),
            }
        }
    }
    fields
}

/// HTML 转义（防 XSS）
pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 组装 HTTP 响应（禁止缓存 + Content-Length 按 UTF-8 字节）
pub fn http_response(status: u16, content_type: &str, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        302 => "Found",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let len = body.as_bytes().len();
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nPragma: no-cache\r\nExpires: 0\r\nConnection: close\r\n\r\n{}",
        status, reason, content_type, len, body
    )
}

/// 302 重定向响应（登录后跳转等）
pub fn redirect(location: &str) -> String {
    let reason = "Found";
    format!(
        "HTTP/1.1 302 {}\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        reason, location
    )
}

/// 带 Set-Cookie 的 302 重定向（登录成功下发会话）
pub fn redirect_with_cookie(location: &str, cookie: &str) -> String {
    let reason = "Found";
    format!(
        "HTTP/1.1 302 {}\r\nLocation: {}\r\nSet-Cookie: {}; Path=/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        reason, location, cookie
    )
}

/// 服务主循环：accept → 解析 → handler → 响应 → 关闭
pub fn serve_loop<F>(listener: TcpListener, handler: F)
where
    F: Fn(&HttpRequest) -> String + Send + Sync + 'static,
{
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
        let read_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut reader = BufReader::new(read_stream);
        let mut writer = stream;
        let response = match parse_request(&mut reader) {
            Some(req) => handler(&req),
            None => http_response(400, "text/plain; charset=utf-8", "Bad Request"),
        };
        let _ = writer.write_all(response.as_bytes());
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_decode_basic() {
        assert_eq!(url_decode("a%20b"), "a b");
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("%E8%AF%BE%E7%A8%8B"), "课程");
    }

    #[test]
    fn parse_form_basic() {
        let fields = parse_form("username=pm1&password=pm123");
        assert_eq!(fields[0].0, "username");
        assert_eq!(fields[0].1, "pm1");
    }

    #[test]
    fn html_escape_works() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn response_headers_correct() {
        let resp = http_response(200, "text/html; charset=utf-8", "你好");
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("Content-Length: 6"));
        assert!(resp.contains("Cache-Control: no-store"));
    }

    #[test]
    fn redirect_format() {
        let r = redirect("/login");
        assert!(r.starts_with("HTTP/1.1 302 Found"));
        assert!(r.contains("Location: /login"));
    }

    #[test]
    fn request_param_prefers_query() {
        let req = HttpRequest {
            method: "GET".into(),
            path: "/x".into(),
            query: vec![("a".into(), "1".into())],
            body: vec![("a".into(), "2".into())],
            cookie: String::new(),
        };
        assert_eq!(req.param("a"), Some("1".to_string()));
    }
}
