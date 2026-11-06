use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub instruction: String,
    pub original: Option<String>,
}

impl Task {
    pub fn shown(&self) -> &str {
        self.original.as_deref().filter(|o| !o.trim().is_empty()).unwrap_or(&self.instruction)
    }
}

fn request(port: u16, method: &str, path: &str, body: Option<&Value>) -> Result<(u16, String), String> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|_| "Saathi is not running on this PC".to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: localhost:{port}\r\nX-Sahay: 1\r\nConnection: close\r\n");
    if body.is_some() {
        req.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\n", payload.len()));
    }
    req.push_str("\r\n");
    req.push_str(&payload);
    stream.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let reply = crate::saathi::parse_http(&String::from_utf8_lossy(&raw))?;
    Ok((reply.status, reply.body))
}

pub fn parse_task(status: u16, body: &str) -> Result<Option<Task>, String> {
    match status {
        204 => Ok(None),
        200 => {
            let v: Value = serde_json::from_str(body).map_err(|_| "Saathi sent a task Sahay could not read".to_string())?;
            let id = v["id"].as_str().unwrap_or_default().to_string();
            let instruction = v["instruction"].as_str().unwrap_or_default().trim().to_string();
            if id.is_empty() || instruction.is_empty() {
                return Err("Saathi sent an empty task".into());
            }
            Ok(Some(Task { id, instruction, original: v["original"].as_str().map(str::to_string) }))
        }
        404 => Ok(None),
        s => Err(format!("Saathi refused to hand out tasks ({s})")),
    }
}

pub fn next(port: u16) -> Result<Option<Task>, String> {
    let (status, body) = request(port, "GET", "/sahay/next", None)?;
    parse_task(status, &body)
}

pub fn report(port: u16, id: &str, status: &str, note: &str) -> Result<(), String> {
    let note: String = note.trim().chars().take(480).collect();
    let (code, body) = request(port, "POST", &format!("/sahay/tasks/{id}"), Some(&json!({ "status": status, "note": note })))?;
    if code >= 400 {
        return Err(format!("Saathi did not accept the update ({code}): {}", body.chars().take(120).collect::<String>()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn tasks_are_read_from_saathi_answers() {
        assert_eq!(parse_task(204, ""), Ok(None));
        assert_eq!(parse_task(404, "{}"), Ok(None));
        let t = parse_task(200, r#"{"id":"a1","instruction":"Open WhatsApp","original":"WhatsApp kaise kholu?","language":"hi-Latn"}"#).unwrap().unwrap();
        assert_eq!(t.id, "a1");
        assert_eq!(t.shown(), "WhatsApp kaise kholu?");
        let plain = parse_task(200, r#"{"id":"a2","instruction":"Print it","original":null}"#).unwrap().unwrap();
        assert_eq!(plain.shown(), "Print it");
        assert!(parse_task(200, r#"{"id":"","instruction":"x"}"#).is_err());
        assert!(parse_task(200, "not json").is_err());
        assert!(parse_task(403, "{}").is_err());
    }

    #[test]
    fn it_takes_a_task_and_reports_back_with_the_sahay_header() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for answer in [
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"id\":\"t1\",\"instruction\":\"Make a WhatsApp group\",\"original\":\"group kaise banau\"}",
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"id\":\"t1\",\"status\":\"done\"}",
                "HTTP/1.1 400 Bad Request\r\n\r\n{\"error\":\"status must be working, done or failed\"}",
                "HTTP/1.1 204 No Content\r\n\r\n",
            ] {
                let (mut s, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap();
                seen.push(String::from_utf8_lossy(&buf[..n]).to_string());
                s.write_all(answer.as_bytes()).unwrap();
            }
            seen
        });
        let task = next(port).unwrap().unwrap();
        assert_eq!(task.instruction, "Make a WhatsApp group");
        report(port, &task.id, "done", "  Made the group Family  ").unwrap();
        assert!(report(port, &task.id, "weird", "").is_err());
        assert_eq!(next(port).unwrap(), None);
        let seen = server.join().unwrap();
        assert!(seen[0].starts_with("GET /sahay/next HTTP/1.1") && seen[0].contains("X-Sahay: 1"));
        assert!(seen[1].starts_with("POST /sahay/tasks/t1") && seen[1].contains("\"note\":\"Made the group Family\""));
        assert!(next(1).is_err());
    }
}
