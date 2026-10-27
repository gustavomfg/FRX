// Explicitly synthetic HTTPS integration fixture. This file is included inside
// tests::file_artifacts so it reuses the existing fake Steam/game installation.
// No network request leaves localhost and no downloaded content is executed.

struct HttpsFixtureServer {
    child: std::process::Child,
    _directory: tempfile::TempDir,
    base_url: String,
    ca_pem: Vec<u8>,
    requests: PathBuf,
}

impl Drop for HttpsFixtureServer {
    fn drop(&mut self) {
        // This is the Python helper process spawned by this test, never Steam.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn https_fixture_openssl(dir: &std::path::Path, arguments: &[&str]) {
    let output = std::process::Command::new("openssl")
        .args(arguments)
        .current_dir(dir)
        .output()
        .expect("This optional fixture requires the installed openssl utility");
    assert!(
        output.status.success(),
        "Controlled certificate generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

impl HttpsFixtureServer {
    fn start(metadata: &[u8], payload: &[u8]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        std::fs::write(path.join("metadata.json"), metadata).unwrap();
        std::fs::write(path.join("payload.ini"), payload).unwrap();
        https_fixture_openssl(
            path,
            &[
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-days",
                "1",
                "-keyout",
                "ca.key",
                "-out",
                "ca.pem",
                "-subj",
                "/CN=FRX synthetic fixture CA",
                "-addext",
                "basicConstraints=critical,CA:TRUE",
                "-addext",
                "keyUsage=critical,keyCertSign,cRLSign",
            ],
        );
        https_fixture_openssl(
            path,
            &[
                "req",
                "-new",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                "server.key",
                "-out",
                "server.csr",
                "-subj",
                "/CN=localhost",
            ],
        );
        std::fs::write(
            path.join("server.ext"),
            "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost\n",
        )
        .unwrap();
        https_fixture_openssl(
            path,
            &[
                "x509",
                "-req",
                "-in",
                "server.csr",
                "-CA",
                "ca.pem",
                "-CAkey",
                "ca.key",
                "-CAcreateserial",
                "-out",
                "server.pem",
                "-days",
                "1",
                "-extfile",
                "server.ext",
            ],
        );
        // The helper serves exactly two known files. It has no general file
        // server, subprocess execution, directory browsing or external access.
        std::fs::write(
            path.join("server.py"),
            r#"import http.server, pathlib, ssl
root = pathlib.Path(__file__).parent
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        name = {'/metadata.json': 'metadata.json', '/payload.ini': 'payload.ini'}.get(self.path)
        if name is None:
            self.send_error(404)
            return
        body = (root / name).read_bytes()
        with (root / 'requests.log').open('a', encoding='utf-8') as log:
            log.write(self.path + '\n')
        self.send_response(200)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Content-Type', 'application/json' if name.endswith('.json') else 'text/plain')
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *_args):
        pass
server = http.server.HTTPServer(('127.0.0.1', 0), Handler)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(root / 'server.pem', root / 'server.key')
server.socket = context.wrap_socket(server.socket, server_side=True)
(root / 'port').write_text(str(server.server_port), encoding='ascii')
server.serve_forever()
"#,
        )
        .unwrap();
        let child = std::process::Command::new("python3")
            .arg(path.join("server.py"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("This optional fixture requires the installed python3 utility");
        let mut server = Self {
            child,
            base_url: String::new(),
            ca_pem: std::fs::read(path.join("ca.pem")).unwrap(),
            requests: path.join("requests.log"),
            _directory: directory,
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Ok(port) = std::fs::read_to_string(server._directory.path().join("port")) {
                let port = port.parse::<u16>().unwrap();
                server.base_url = format!("https://localhost:{port}");
                break;
            }
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "Controlled HTTPS helper exited before listening"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "HTTPS fixture startup timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        server
    }

    fn client(&self) -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .no_proxy()
            .https_only(true)
            .add_root_certificate(reqwest::Certificate::from_pem(&self.ca_pem).unwrap())
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(2))
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap()
    }
}

struct HttpsFixtureTransport {
    base_url: String,
    asset_url: String,
    client: reqwest::blocking::Client,
}

impl artifacts::Transport for HttpsFixtureTransport {
    fn get(
        &self,
        url: &str,
        limit: usize,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<Vec<u8>, String> {
        use std::io::Read;
        artifacts::cancelled(cancel)?;
        // Translation is test-only. Production HTTPS and host policies are
        // unchanged; the two canonical fixture identities remain revalidated.
        if !artifacts::allowed_url(url) {
            return Err("Noncanonical fixture URL".into());
        }
        let path = if url == META {
            "/metadata.json"
        } else if url == self.asset_url {
            "/payload.ini"
        } else {
            return Err("URL absent from the controlled HTTPS fixture".into());
        };
        let mut response = self
            .client
            .get(format!("{}{path}", self.base_url))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|e| format!("Controlled HTTPS request failed: {e}"))?;
        let total = response.content_length();
        if total.is_some_and(|n| n > limit as u64) {
            return Err("Fixture response exceeds byte limit".into());
        }
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            artifacts::cancelled(cancel)?;
            let count = response.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            if bytes.len() + count > limit {
                return Err("Fixture response exceeds byte limit".into());
            }
            bytes.extend_from_slice(&buffer[..count]);
            progress(bytes.len() as u64, total);
        }
        artifacts::cancelled(cancel)?;
        Ok(bytes)
    }
}

#[test]
#[ignore = "Controlled localhost TLS integration: requires openssl, python3 and local sockets"]
fn controlled_https_download_install_restart_and_rollback() {
    let original = b"[synthetic-fixture]\nvalue=before\n";
    let payload = b"[synthetic-fixture]\nvalue=after\n";
    let fx = fixture();
    let target = fx.library.join("Config/frx.ini");
    std::fs::write(&target, original).unwrap();
    let (candidate, controlled) = direct(payload, Some(original));
    let server = HttpsFixtureServer::start(&controlled.responses[META], payload);

    let untrusted_client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();
    assert!(
        untrusted_client
            .get(format!("{}/metadata.json", server.base_url))
            .send()
            .is_err(),
        "A private fixture CA must not be accepted by the default TLS trust store"
    );
    assert!(
        server
            .client()
            .get(format!(
                "{}/metadata.json",
                server.base_url.replace("localhost", "127.0.0.1")
            ))
            .send()
            .is_err(),
        "Certificate hostname validation must remain enabled"
    );
    let transport = HttpsFixtureTransport {
        base_url: server.base_url.clone(),
        asset_url: candidate.asset_url.clone(),
        client: server.client(),
    };
    let (mut engine, game, id) = setup(&fx, candidate, controlled);
    engine.artifact_transport = Arc::new(transport);
    let plan = engine.prepare(&id, &game).unwrap();
    assert_eq!(plan.status, PlanStatus::Ready, "{:?}", plan.reasons);
    assert_eq!(plan.artifacts.len(), 1);
    assert_eq!(std::fs::read(&target).unwrap(), original);
    assert!(
        !server.requests.exists(),
        "Preview must not download the payload"
    );

    let states = std::sync::Mutex::new(Vec::new());
    let operation = engine
        .apply(&plan.plan_id, &plan.fingerprint, &game, &|op| {
            states.lock().unwrap().push(op.state);
        })
        .unwrap();
    assert_eq!(operation.state, OpState::Applied, "{:?}", operation.message);
    assert!(states.lock().unwrap().contains(&OpState::Downloading));
    assert!(states.lock().unwrap().contains(&OpState::Verifying));
    assert_eq!(std::fs::read(&target).unwrap(), payload);
    let backup = operation.steps[0].backup.as_ref().unwrap();
    assert!(backup.existed);
    assert_eq!(std::fs::read(&backup.path).unwrap(), original);
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
    assert_eq!(
        std::fs::read_to_string(&server.requests).unwrap(),
        "/metadata.json\n/payload.ini\n"
    );

    drop(engine);
    let restarted = fx.engine();
    let persisted = restarted.operation(&operation.id).unwrap();
    assert_eq!(persisted.state, OpState::Applied);
    assert_eq!(persisted.artifacts, operation.artifacts);
    assert_eq!(
        persisted.steps[0].backup.as_ref().unwrap().sha256,
        backup.sha256
    );
    let rollback = restarted.prepare_rollback(&operation.id).unwrap();
    assert_eq!(rollback.status, PlanStatus::Ready);
    assert_eq!(
        restarted
            .rollback(&operation.id, &rollback.fingerprint, &noop)
            .unwrap()
            .state,
        OpState::RolledBack
    );
    assert_eq!(std::fs::read(&target).unwrap(), original);
    assert_eq!(std::fs::read(&fx.config).unwrap(), LOCALCONFIG.as_bytes());
    assert_eq!(
        fx.engine().operation(&operation.id).unwrap().state,
        OpState::RolledBack
    );
    println!("Synthetic localhost HTTPS fixture: TLS trust + hostname checks, preview, metadata, payload, checksum, backup, install, persisted history and rollback passed. No real game or benchmark used.");
}
