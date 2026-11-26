//! A WebDAV folder (Nextcloud, ownCloud, any server): PROPFIND with depth 1 lists it, GET
//! reads a file, PUT writes one whole.

use std::time::Duration;

use base64::Engine;
use ureq::http::Request;

use crate::{FILE_END, Remote, Result, SyncError};

/// How long one request may take before the sync gives up.
const WAIT: Duration = Duration::from_secs(30);

/// The status a PROPFIND answers with (RFC 4918).
const MULTI_STATUS: u16 = 207;

/// A folder on a WebDAV server, e.g. `https://cloud.example.org/remote.php/dav/files/jonas/Kaiwa/`.
pub struct WebDav {
    /// Ends in `/`.
    url: String,
    auth: String,
    agent: ureq::Agent,
}

impl WebDav {
    pub fn new(url: &str, user: &str, password: &str) -> Self {
        let url = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{url}/")
        };
        let pair = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        let config = ureq::Agent::config_builder()
            .allow_non_standard_methods(true)
            .http_status_as_error(false)
            .timeout_global(Some(WAIT))
            .build();
        Self {
            url,
            auth: format!("Basic {pair}"),
            agent: ureq::Agent::new_with_config(config),
        }
    }

    /// Sends `method` to `name` in the folder (the folder itself for ""); the status and body.
    fn send(&self, method: &str, name: &str, body: &[u8]) -> Result<(u16, Vec<u8>)> {
        let mut req = Request::builder()
            .method(method)
            .uri(format!("{}{name}", self.url))
            .header("Authorization", &self.auth);
        if method == "PROPFIND" {
            req = req.header("Depth", "1");
        }
        let req = req.body(body.to_vec()).map_err(|e| net(e.to_string()))?;
        let mut res = self.agent.run(req).map_err(|e| net(e.to_string()))?;
        let status = res.status().as_u16();
        let body = res
            .body_mut()
            .read_to_vec()
            .map_err(|e| net(e.to_string()))?;
        Ok((status, body))
    }

    /// `name` answered `status`, which is not what a working server answers.
    fn refused(&self, name: &str, status: u16) -> SyncError {
        net(format!("{}{name}: HTTP {status}", self.url))
    }
}

fn net(e: String) -> SyncError {
    SyncError::Io(std::io::Error::other(e))
}

fn ok(status: u16) -> bool {
    (200..300).contains(&status)
}

impl Remote for WebDav {
    fn list(&self) -> Result<Vec<String>> {
        let (status, body) = self.send("PROPFIND", "", b"")?;
        if status != MULTI_STATUS {
            return Err(self.refused("", status));
        }
        Ok(names(&String::from_utf8_lossy(&body)))
    }

    fn get(&self, name: &str) -> Result<Vec<u8>> {
        let (status, body) = self.send("GET", name, b"")?;
        if !ok(status) {
            return Err(self.refused(name, status));
        }
        Ok(body)
    }

    fn put(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let (status, _) = self.send("PUT", name, bytes)?;
        if !ok(status) {
            return Err(self.refused(name, status));
        }
        Ok(())
    }
}

/// The journal files a PROPFIND answer names: the last part of every `href`, e.g.
/// `<d:href>/dav/Kaiwa/3f9a.kaiwa-sync</d:href>` gives `3f9a.kaiwa-sync`.
fn names(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in xml.split("href>").skip(1).step_by(2) {
        let Some(href) = part.split('<').next() else {
            continue;
        };
        let Some(name) = href.trim_end_matches('/').rsplit('/').next() else {
            continue;
        };
        if name.ends_with(FILE_END) && !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// The server's files, by name.
    type Files = Mutex<Vec<(String, Vec<u8>)>>;

    /// A WebDAV server for one folder in a thread: it answers PROPFIND, GET and PUT and
    /// refuses a wrong login.
    fn server(auth: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/dav/Kaiwa/", listener.local_addr().unwrap());
        let files: Arc<Files> = Arc::default();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let (status, body) = answer(&mut stream, auth, &files);
                let head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        url
    }

    fn answer(stream: &mut std::net::TcpStream, auth: &str, files: &Files) -> (u16, Vec<u8>) {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let mut parts = line.split_whitespace();
        let (method, path) = (
            parts.next().unwrap().to_string(),
            parts.next().unwrap().to_string(),
        );
        let (mut length, mut login) = (0, String::new());
        loop {
            let mut h = String::new();
            reader.read_line(&mut h).unwrap();
            if h.trim().is_empty() {
                break;
            }
            let (k, v) = h.split_once(':').unwrap();
            match k.to_ascii_lowercase().as_str() {
                "content-length" => length = v.trim().parse().unwrap(),
                "authorization" => login = v.trim().to_string(),
                _ => {}
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        if login != auth {
            return (401, Vec::new());
        }
        let name = path.rsplit('/').next().unwrap_or_default().to_string();
        let mut files = files.lock().unwrap();
        match method.as_str() {
            "PROPFIND" => {
                let mut xml = String::from(
                    "<d:multistatus><d:response><d:href>/dav/Kaiwa/</d:href></d:response>",
                );
                for (n, _) in files.iter() {
                    xml += &format!("<d:response><d:href>/dav/Kaiwa/{n}</d:href></d:response>");
                }
                xml += "<d:response><d:href>/dav/Kaiwa/notes.txt</d:href></d:response></d:multistatus>";
                (MULTI_STATUS, xml.into_bytes())
            }
            "GET" => match files.iter().find(|(n, _)| *n == name) {
                Some((_, b)) => (200, b.clone()),
                None => (404, Vec::new()),
            },
            "PUT" => {
                files.retain(|(n, _)| *n != name);
                files.push((name, body));
                (201, Vec::new())
            }
            _ => (405, Vec::new()),
        }
    }

    #[test]
    fn a_webdav_folder_lists_reads_and_writes_journal_files() {
        let auth = "Basic am9uYXM6Z2VoZWlt"; // jonas:geheim
        let dav = WebDav::new(server(auth).trim_end_matches('/'), "jonas", "geheim");
        assert!(dav.list().unwrap().is_empty());
        dav.put("b.kaiwa-sync", b"1").unwrap();
        dav.put("a.kaiwa-sync", b"2").unwrap();
        dav.put("a.kaiwa-sync", b"3").unwrap();
        assert_eq!(dav.list().unwrap(), ["a.kaiwa-sync", "b.kaiwa-sync"]);
        assert_eq!(dav.get("a.kaiwa-sync").unwrap(), b"3");
        assert!(dav.get("gone.kaiwa-sync").is_err());
    }

    #[test]
    fn a_wrong_login_is_an_error() {
        let url = server("Basic am9uYXM6Z2VoZWlt");
        let dav = WebDav::new(&url, "jonas", "falsch");
        let e = dav.list().unwrap_err().to_string();
        assert!(e.contains("401"), "{e}");
    }
}
