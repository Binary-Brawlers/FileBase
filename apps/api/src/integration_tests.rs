//! End-to-end API tests that drive the real router against PostgreSQL.
//!
//! Each test creates its own throwaway database (so tests run in parallel and
//! first-run setup can be exercised every time) plus a temp upload directory,
//! and drops both when it finishes.
//!
//! Requires `FILEBASE_TEST_DATABASE_URL`, a URL for a role allowed to
//! `CREATE DATABASE`, e.g. `postgres://postgres:postgres@localhost:5432/postgres`.
//! Without it every test here logs a skip message and passes; CI sets it.

use std::path::PathBuf;

use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, Method, Request, StatusCode},
    Router,
};
use sea_orm::{ConnectionTrait, Database};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::app;
use crate::config::Config;

const ADMIN_EMAIL: &str = "admin@example.test";
const ADMIN_PASSWORD: &str = "admin-password-123";
const PRESET: &str = "docs";

struct TestApp {
    router: Router,
    upload_dir: PathBuf,
    // Declared after `router` so the app's pool is dropped first.
    _database: TestDatabase,
}

/// Owns a per-test database and upload directory and removes both when
/// dropped, so cleanup also runs when a test assertion or the app build panics.
struct TestDatabase {
    admin_url: String,
    database_name: String,
    upload_dir: PathBuf,
}

struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

/// A fully initialized instance: admin logged in, one project with a local
/// storage connection and the `docs` preset (txt/png, hash names, return_existing).
struct Seeded {
    app: TestApp,
    project_id: String,
    admin_token: String,
}

/// Creates a fresh database and builds the app against it, or returns `None`
/// when no test database is configured.
async fn start_app(customize: impl FnOnce(&mut Config)) -> Option<TestApp> {
    let admin_url = match std::env::var("FILEBASE_TEST_DATABASE_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            eprintln!("skipping: FILEBASE_TEST_DATABASE_URL is not set");
            return None;
        }
    };
    let database_name = format!("filebase_test_{}", Uuid::new_v4().simple());
    run_admin_sql(&admin_url, &format!("CREATE DATABASE \"{database_name}\"")).await;
    let upload_dir = std::env::temp_dir().join(&database_name);
    let database = TestDatabase {
        admin_url: admin_url.clone(),
        database_name: database_name.clone(),
        upload_dir: upload_dir.clone(),
    };

    let mut config = Config::for_tests(
        &test_database_url(&admin_url, &database_name),
        &upload_dir.to_string_lossy(),
    );
    customize(&mut config);
    let built = match app::build(config).await {
        Ok(built) => built,
        Err(error) => panic!("app build failed: {error:?}"),
    };
    Some(TestApp {
        router: built.router,
        upload_dir,
        _database: database,
    })
}

/// Swaps the database name in a Postgres URL, keeping credentials, host and
/// any query parameters (e.g. `?sslrootcert=/certs/root.crt`) intact.
fn test_database_url(admin_url: &str, database: &str) -> String {
    let (base, query) = match admin_url.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (admin_url, None),
    };
    let server = match base.rsplit_once('/') {
        Some((server, _database)) if server.contains("://") => server,
        _ => panic!("FILEBASE_TEST_DATABASE_URL must end with /<database>"),
    };
    match query {
        Some(query) => format!("{server}/{database}?{query}"),
        None => format!("{server}/{database}"),
    }
}

async fn run_admin_sql(admin_url: &str, sql: &str) {
    let connection = match Database::connect(admin_url).await {
        Ok(connection) => connection,
        Err(error) => panic!("cannot connect to test database server: {error}"),
    };
    if let Err(error) = connection.execute_unprepared(sql).await {
        panic!("admin sql failed ({sql}): {error}");
    }
    let _ = connection.close().await;
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // `Drop` cannot await, and the test's runtime may be unwinding, so the
        // DROP runs on a short-lived thread with its own runtime. Errors are
        // only logged: panicking here during an unwind would abort the run.
        let admin_url = self.admin_url.clone();
        let sql = format!(
            "DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)",
            self.database_name
        );
        let cleanup = std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("test database cleanup: runtime failed: {error}");
                    return;
                }
            };
            runtime.block_on(async {
                match Database::connect(&admin_url).await {
                    Ok(connection) => {
                        if let Err(error) = connection.execute_unprepared(&sql).await {
                            eprintln!("test database cleanup failed ({sql}): {error}");
                        }
                        let _ = connection.close().await;
                    }
                    Err(error) => eprintln!("test database cleanup: connect failed: {error}"),
                }
            });
        });
        if cleanup.join().is_err() {
            eprintln!("test database cleanup thread panicked");
        }
        let _ = std::fs::remove_dir_all(&self.upload_dir);
    }
}

impl TestApp {
    async fn send(&self, request: Request<Body>) -> Response {
        let response = match self.router.clone().oneshot(request).await {
            Ok(response) => response,
            Err(error) => panic!("router error: {error}"),
        };
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = match to_bytes(response.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(error) => panic!("reading response body failed: {error}"),
        };
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            match serde_json::from_slice(&bytes) {
                Ok(value) => value,
                Err(_) => Value::String(String::from_utf8_lossy(&bytes).into_owned()),
            }
        };
        Response {
            status,
            headers,
            body,
        }
    }

    async fn json(&self, method: Method, uri: &str, token: Option<&str>, body: Value) -> Response {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let request = match builder.body(Body::from(body.to_string())) {
            Ok(request) => request,
            Err(error) => panic!("request build failed: {error}"),
        };
        self.send(request).await
    }

    async fn get(&self, uri: &str, token: Option<&str>) -> Response {
        let mut builder = Request::builder().method(Method::GET).uri(uri);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        match builder.body(Body::empty()) {
            Ok(request) => self.send(request).await,
            Err(error) => panic!("request build failed: {error}"),
        }
    }

    /// Sends a multipart form; `file` is `(field, filename, content_type, bytes)`.
    async fn multipart(
        &self,
        uri: &str,
        token: Option<&str>,
        fields: &[(&str, &str)],
        file: Option<(&str, &str, &str, &[u8])>,
    ) -> Response {
        let boundary = "filebase-test-boundary";
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                )
                .as_bytes(),
            );
        }
        if let Some((field, filename, content_type, bytes)) = file {
            body.extend_from_slice(
                format!(
                    "--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(bytes);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

        let mut builder = Request::builder().method(Method::POST).uri(uri).header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        );
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        match builder.body(Body::from(body)) {
            Ok(request) => self.send(request).await,
            Err(error) => panic!("request build failed: {error}"),
        }
    }

    async fn upload_text(&self, api_key: &str, filename: &str, contents: &[u8]) -> Response {
        self.multipart(
            "/uploads",
            Some(api_key),
            &[("preset", PRESET)],
            Some(("file", filename, "text/plain", contents)),
        )
        .await
    }
}

fn text(value: &Value, pointer: &str) -> String {
    match value.pointer(pointer).and_then(Value::as_str) {
        Some(text) => text.to_string(),
        None => panic!("missing string at {pointer} in {value}"),
    }
}

fn setup_payload(upload_dir: &str, duplicate_strategy: &str) -> Value {
    json!({
        "admin": { "name": "Admin", "email": ADMIN_EMAIL, "password": ADMIN_PASSWORD },
        "project": { "name": "Test Project" },
        "storage": {
            "type": "local",
            "base_path": upload_dir,
            "public_base_url": "http://files.test/"
        },
        "preset": {
            "name": PRESET,
            "folder": "docs",
            "allowed_mime_types": ["text/plain", "image/png"],
            "allowed_extensions": ["txt", "png"],
            "max_file_size": 1024,
            "duplicate_strategy": duplicate_strategy,
            "filename_strategy": "hash"
        }
    })
}

async fn login(app: &TestApp, email: &str, password: &str) -> Response {
    app.json(
        Method::POST,
        "/auth/login",
        None,
        json!({ "email": email, "password": password }),
    )
    .await
}

async fn seed(app: TestApp) -> Seeded {
    let upload_dir = app.upload_dir.to_string_lossy().into_owned();
    let setup = app
        .json(
            Method::POST,
            "/setup/initialize",
            None,
            setup_payload(&upload_dir, "return_existing"),
        )
        .await;
    assert_eq!(setup.status, StatusCode::OK, "setup failed: {}", setup.body);
    let project_id = text(&setup.body, "/data/project_id");
    let logged_in = login(&app, ADMIN_EMAIL, ADMIN_PASSWORD).await;
    assert_eq!(logged_in.status, StatusCode::OK, "{}", logged_in.body);
    let admin_token = text(&logged_in.body, "/data/token");
    Seeded {
        app,
        project_id,
        admin_token,
    }
}

async fn create_api_key(seeded: &Seeded) -> (String, String) {
    let created = seeded
        .app
        .json(
            Method::POST,
            "/api-keys",
            Some(&seeded.admin_token),
            json!({ "project_id": seeded.project_id, "name": "tests", "mode": "test" }),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    (
        text(&created.body, "/data/id"),
        text(&created.body, "/data/secret"),
    )
}

#[tokio::test]
async fn setup_runs_once_and_then_locks() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let status = app.get("/setup/status", None).await;
    assert_eq!(status.body["data"]["setup_required"], json!(true));

    let upload_dir = app.upload_dir.to_string_lossy().into_owned();
    let first = app
        .json(
            Method::POST,
            "/setup/initialize",
            None,
            setup_payload(&upload_dir, "return_existing"),
        )
        .await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    for field in [
        "user_id",
        "project_id",
        "storage_connection_id",
        "upload_preset_id",
    ] {
        assert!(first.body["data"][field].is_string(), "missing {field}");
    }

    let status = app.get("/setup/status", None).await;
    assert_eq!(status.body["data"]["setup_required"], json!(false));

    let second = app
        .json(
            Method::POST,
            "/setup/initialize",
            None,
            setup_payload(&upload_dir, "return_existing"),
        )
        .await;
    assert_eq!(second.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn failed_setup_leaves_the_instance_uninitialized() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let upload_dir = app.upload_dir.to_string_lossy().into_owned();
    // The preset is validated after the admin user is inserted; the whole
    // setup must roll back so the operator can retry.
    let invalid = app
        .json(
            Method::POST,
            "/setup/initialize",
            None,
            setup_payload(&upload_dir, "keep_both"),
        )
        .await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);

    let status = app.get("/setup/status", None).await;
    assert_eq!(status.body["data"]["setup_required"], json!(true));
    let login_attempt = login(&app, ADMIN_EMAIL, ADMIN_PASSWORD).await;
    assert_eq!(login_attempt.status, StatusCode::UNAUTHORIZED);

    let retry = app
        .json(
            Method::POST,
            "/setup/initialize",
            None,
            setup_payload(&upload_dir, "return_existing"),
        )
        .await;
    assert_eq!(retry.status, StatusCode::OK, "{}", retry.body);
}

#[tokio::test]
async fn login_session_and_logout() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let app = &seeded.app;

    let logged_in = login(app, "ADMIN@example.test", ADMIN_PASSWORD).await;
    assert_eq!(
        logged_in.status,
        StatusCode::OK,
        "emails are case-insensitive"
    );
    let cookie = match logged_in.headers.get(header::SET_COOKIE) {
        Some(value) => match value.to_str() {
            Ok(cookie) => cookie.to_string(),
            Err(error) => panic!("session cookie is not ASCII: {error}"),
        },
        None => panic!("login did not set a session cookie"),
    };
    assert!(cookie.contains("filebase_session="));
    assert!(cookie.to_ascii_lowercase().contains("httponly"));

    let me = app.get("/auth/me", Some(&seeded.admin_token)).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(text(&me.body, "/data/email"), ADMIN_EMAIL);
    assert!(me.body["data"].get("password_hash").is_none());

    assert_eq!(
        app.get("/auth/me", None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.get("/auth/me", Some("garbage")).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(app, ADMIN_EMAIL, "wrong-password").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(app, "nobody@example.test", ADMIN_PASSWORD)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    let logout = app
        .json(
            Method::POST,
            "/auth/logout",
            Some(&seeded.admin_token),
            json!({}),
        )
        .await;
    assert!(logout.status.is_success(), "logout: {}", logout.status);
}

#[tokio::test]
async fn direct_upload_stores_file_and_returns_existing_duplicate() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let (_key_id, api_key) = create_api_key(&seeded).await;
    let app = &seeded.app;

    let first = app
        .upload_text(&api_key, "notes.txt", b"hello filebase")
        .await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);
    let file_id = text(&first.body, "/data/id");
    let path = text(&first.body, "/data/path");
    let hash = text(&first.body, "/data/hash");
    assert_eq!(path, format!("docs/{hash}.txt"));
    assert_eq!(
        text(&first.body, "/data/url"),
        format!("http://files.test/{path}")
    );
    assert_eq!(first.body["data"]["duplicate"], json!(false));
    match std::fs::read(app.upload_dir.join(&path)) {
        Ok(bytes) => assert_eq!(bytes, b"hello filebase"),
        Err(error) => panic!("uploaded file missing on disk: {error}"),
    }

    let second = app
        .upload_text(&api_key, "renamed.txt", b"hello filebase")
        .await;
    assert_eq!(second.status, StatusCode::CREATED, "{}", second.body);
    assert_eq!(text(&second.body, "/data/id"), file_id);

    let fetched = app
        .get(&format!("/files/{file_id}"), Some(&seeded.admin_token))
        .await;
    assert_eq!(fetched.status, StatusCode::OK);
    assert_eq!(text(&fetched.body, "/data/original_name"), "notes.txt");
}

#[tokio::test]
async fn uploads_are_validated_against_key_and_preset() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let (key_id, api_key) = create_api_key(&seeded).await;
    let app = &seeded.app;

    assert_eq!(
        app.upload_text("fb_not_a_real_key", "a.txt", b"x")
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.upload_text(&api_key, "script.exe", b"x").await.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "extension outside the preset allow-list"
    );
    let png_bytes: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    assert_eq!(
        app.upload_text(&api_key, "fake.txt", png_bytes)
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "PNG bytes declared as text/plain"
    );
    assert_eq!(
        app.upload_text(&api_key, "big.txt", &[b'a'; 2048])
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "larger than the preset max_file_size"
    );
    let unknown_preset = app
        .multipart(
            "/uploads",
            Some(&api_key),
            &[("preset", "missing")],
            Some(("file", "a.txt", "text/plain", b"x")),
        )
        .await;
    assert_eq!(unknown_preset.status, StatusCode::NOT_FOUND);
    let no_file = app
        .multipart("/uploads", Some(&api_key), &[("preset", PRESET)], None)
        .await;
    assert_eq!(no_file.status, StatusCode::UNPROCESSABLE_ENTITY);

    let revoked = app
        .json(
            Method::PATCH,
            &format!("/api-keys/{key_id}/revoke"),
            Some(&seeded.admin_token),
            json!({}),
        )
        .await;
    assert!(revoked.status.is_success(), "revoke: {}", revoked.body);
    assert_eq!(
        app.upload_text(&api_key, "a.txt", b"after revoke")
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn reject_duplicate_preset_returns_conflict() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let (_key_id, api_key) = create_api_key(&seeded).await;
    let app = &seeded.app;
    let preset = app
        .json(
            Method::POST,
            "/upload-presets",
            Some(&seeded.admin_token),
            json!({
                "project_id": seeded.project_id,
                "name": "strict",
                "folder": "strict",
                "allowed_extensions": ["txt"],
                "max_file_size": 1024,
                "duplicate_strategy": "reject_duplicate",
                "filename_strategy": "uuid"
            }),
        )
        .await;
    assert_eq!(preset.status, StatusCode::CREATED, "{}", preset.body);

    let upload = |contents: &'static [u8]| {
        app.multipart(
            "/uploads",
            Some(&api_key),
            &[("preset", "strict")],
            Some(("file", "a.txt", "text/plain", contents)),
        )
    };
    assert_eq!(upload(b"unique body").await.status, StatusCode::CREATED);
    assert_eq!(upload(b"unique body").await.status, StatusCode::CONFLICT);
    assert_eq!(upload(b"different body").await.status, StatusCode::CREATED);
}

#[tokio::test]
async fn signed_upload_session_is_single_use() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let (_key_id, api_key) = create_api_key(&seeded).await;
    let app = &seeded.app;
    let signed = app
        .json(
            Method::POST,
            "/uploads/sign",
            Some(&api_key),
            json!({ "preset": PRESET }),
        )
        .await;
    assert_eq!(signed.status, StatusCode::CREATED, "{}", signed.body);
    let session_id = text(&signed.body, "/data/id");
    let token = text(&signed.body, "/data/token");
    assert_eq!(
        text(&signed.body, "/data/upload_url"),
        format!("http://api.test/uploads/{session_id}")
    );
    let uri = format!("/uploads/{session_id}");
    let file = Some((
        "file",
        "browser.txt",
        "text/plain",
        b"from the browser".as_slice(),
    ));

    let wrong_token = app
        .multipart(&uri, Some("fb_upload_wrong"), &[], file)
        .await;
    assert_eq!(wrong_token.status, StatusCode::UNAUTHORIZED);

    let uploaded = app.multipart(&uri, Some(&token), &[], file).await;
    assert_eq!(uploaded.status, StatusCode::CREATED, "{}", uploaded.body);
    assert_eq!(text(&uploaded.body, "/data/original_name"), "browser.txt");

    let reused = app.multipart(&uri, Some(&token), &[], file).await;
    assert_eq!(reused.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn chunked_upload_assembles_chunks_in_order() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let (_key_id, api_key) = create_api_key(&seeded).await;
    let app = &seeded.app;
    let signed = app
        .json(
            Method::POST,
            "/uploads/sign",
            Some(&api_key),
            json!({ "preset": PRESET }),
        )
        .await;
    assert_eq!(signed.status, StatusCode::CREATED, "{}", signed.body);
    let session_id = text(&signed.body, "/data/id");
    let token = text(&signed.body, "/data/token");
    let chunks_uri = format!("/uploads/{session_id}/chunks");

    // Send out of order to prove assembly sorts by index.
    for (index, bytes) in [("1", b"world".as_slice()), ("0", b"hello ".as_slice())] {
        let accepted = app
            .multipart(
                &chunks_uri,
                Some(&token),
                &[("chunk_index", index)],
                Some(("chunk", "blob", "application/octet-stream", bytes)),
            )
            .await;
        assert_eq!(accepted.status, StatusCode::CREATED, "{}", accepted.body);
    }
    let status = app.get(&chunks_uri, Some(&token)).await;
    assert_eq!(status.body["data"]["received_chunks"], json!([0, 1]));
    assert_eq!(status.body["data"]["received_bytes"], json!(11));

    let completed = app
        .json(
            Method::POST,
            &format!("/uploads/{session_id}/complete"),
            Some(&token),
            json!({ "filename": "joined.txt", "content_type": "text/plain" }),
        )
        .await;
    assert_eq!(completed.status, StatusCode::CREATED, "{}", completed.body);
    let path = text(&completed.body, "/data/path");
    match std::fs::read(app.upload_dir.join(&path)) {
        Ok(bytes) => assert_eq!(bytes, b"hello world"),
        Err(error) => panic!("assembled file missing on disk: {error}"),
    }
}

#[tokio::test]
async fn viewer_role_can_read_but_not_administer_a_project() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let app = &seeded.app;
    let invited = app
        .json(
            Method::POST,
            &format!("/projects/{}/invitations", seeded.project_id),
            Some(&seeded.admin_token),
            json!({ "email": "viewer@example.test", "role": "viewer" }),
        )
        .await;
    assert_eq!(invited.status, StatusCode::CREATED, "{}", invited.body);
    let invite_token = text(&invited.body, "/data/accept_token");

    let owner_invite = app
        .json(
            Method::POST,
            &format!("/projects/{}/invitations", seeded.project_id),
            Some(&seeded.admin_token),
            json!({ "email": "owner@example.test", "role": "owner" }),
        )
        .await;
    assert_eq!(owner_invite.status, StatusCode::UNPROCESSABLE_ENTITY);

    let accepted = app
        .json(
            Method::POST,
            "/team-invitations/accept",
            None,
            json!({ "token": invite_token, "name": "Viewer", "password": "viewer-password" }),
        )
        .await;
    assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
    let viewer_token = text(&accepted.body, "/data/token");

    let project = app
        .get(
            &format!("/projects/{}", seeded.project_id),
            Some(&viewer_token),
        )
        .await;
    assert_eq!(project.status, StatusCode::OK);
    let key_attempt = app
        .json(
            Method::POST,
            "/api-keys",
            Some(&viewer_token),
            json!({ "project_id": seeded.project_id, "name": "nope", "mode": "test" }),
        )
        .await;
    assert_eq!(key_attempt.status, StatusCode::FORBIDDEN);

    let reused = app
        .json(
            Method::POST,
            "/team-invitations/accept",
            None,
            json!({ "token": invite_token, "name": "Viewer", "password": "viewer-password" }),
        )
        .await;
    assert!(
        reused.status.is_client_error(),
        "accepted invitation must not be reusable: {}",
        reused.status
    );
}

#[tokio::test]
async fn storage_connection_responses_never_include_credentials() {
    let Some(app) = start_app(|_| {}).await else {
        return;
    };
    let seeded = seed(app).await;
    let app = &seeded.app;
    let created = app
        .json(
            Method::POST,
            "/storage-connections",
            Some(&seeded.admin_token),
            json!({
                "project_id": seeded.project_id,
                "type": "ftp",
                "host": "ftp.example.test",
                "username": "uploader",
                "password": "super-secret-ftp-password",
                "base_path": "/public_html/uploads",
                "public_base_url": "https://example.test/uploads"
            }),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["data"]["has_password"], json!(true));
    let connection_id = text(&created.body, "/data/id");

    let listed = app
        .get("/storage-connections", Some(&seeded.admin_token))
        .await;
    let fetched = app
        .get(
            &format!("/storage-connections/{connection_id}"),
            Some(&seeded.admin_token),
        )
        .await;
    for response in [&created, &listed, &fetched] {
        let serialized = response.body.to_string();
        assert!(!serialized.contains("super-secret-ftp-password"));
        assert!(!serialized.contains("encrypted_password"));
    }
}

#[tokio::test]
async fn login_is_rate_limited_per_client() {
    let Some(app) = start_app(|config| config.auth_rate_limit_per_minute = 3).await else {
        return;
    };
    let seeded = seed(app).await;
    let app = &seeded.app;
    // `seed` already used two auth requests (setup + login) from the same client.
    assert_eq!(
        login(app, ADMIN_EMAIL, "wrong-password").await.status,
        StatusCode::UNAUTHORIZED
    );
    let limited = login(app, ADMIN_EMAIL, ADMIN_PASSWORD).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(text(&limited.body, "/error/code"), "rate_limited");
}

#[test]
fn test_database_url_replaces_only_the_database_name() {
    assert_eq!(
        test_database_url("postgres://u:p@localhost:5432/postgres", "t1"),
        "postgres://u:p@localhost:5432/t1"
    );
    assert_eq!(
        test_database_url(
            "postgres://u@db/postgres?sslmode=verify-full&sslrootcert=/certs/root.crt",
            "t1"
        ),
        "postgres://u@db/t1?sslmode=verify-full&sslrootcert=/certs/root.crt"
    );
}
