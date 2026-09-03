use std::{
    collections::HashMap,
    io::Write,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use async_trait::async_trait;
use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use reqwest::{
    Method, StatusCode, Url,
    header::{HeaderMap, HeaderName, HeaderValue, LOCATION},
};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    icy_board::icb_config::{PplHttpDestinationPolicy, PplHttpOptions, normalize_ppl_http_origin},
    parser::{HTTP_ID, HTTP_METHOD_ENUM_ID, HTTP_REQUEST_ID, HTTP_RESPONSE_ID},
};

use super::ppl_error::{ERR_DENIED, ERR_FORMAT, ERR_INVALID, ERR_IO, ERR_KIND_NET, ERR_LIMIT, ERR_TIMEOUT, ERR_UNAVAILABLE, ERR_UNSUPPORTED, PplError};

macro_rules! member_name {
    ($constant:ident, $name:literal) => {
        static $constant: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($name.to_string()));
    };
}

member_name!(GET, "Get");
member_name!(NEW, "New");
member_name!(DOWNLOAD, "Download");
member_name!(URL_ENCODE, "UrlEncode");
member_name!(URL_DECODE, "UrlDecode");
member_name!(VALID, "Valid");
member_name!(OK, "OK");
member_name!(STATUS, "Status");
member_name!(FINAL_URL, "FinalUrl");
member_name!(SIZE, "Size");
member_name!(CONTENT_TYPE, "ContentType");
member_name!(TEXT, "Text");
member_name!(HEADER, "Header");
member_name!(SAVE, "Save");
member_name!(URL, "Url");
member_name!(METHOD, "Method");
member_name!(SET_HEADER, "SetHeader");
member_name!(SET_TEXT, "SetText");
member_name!(SET_FORM, "SetForm");
member_name!(SEND, "Send");

const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

/// Everything outside the RFC 3986 unreserved set (`ALPHA / DIGIT / -._~`).
const URI_COMPONENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'\x7f');

fn url_encode(text: &str, form: bool) -> String {
    if form {
        form_urlencoded::byte_serialize(text.as_bytes()).collect()
    } else {
        utf8_percent_encode(text, URI_COMPONENT).to_string()
    }
}

fn url_decode(text: &str, form: bool) -> String {
    if form {
        percent_decode_str(&text.replace('+', " ")).decode_utf8_lossy().into_owned()
    } else {
        percent_decode_str(text).decode_utf8_lossy().into_owned()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PplHttp;

impl PplHttp {
    fn value() -> VariableValue {
        user_data_value(PplHttp, HTTP_ID)
    }
}

#[derive(Clone, Debug)]
struct HttpRequestData {
    method: Method,
    url: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

#[derive(Debug)]
pub struct PplHttpRequest {
    request: Mutex<HttpRequestData>,
}

impl PplHttpRequest {
    fn new(method: Method, url: String) -> Self {
        Self {
            request: Mutex::new(HttpRequestData {
                method,
                url,
                headers: HeaderMap::new(),
                body: Vec::new(),
            }),
        }
    }

    fn request(&self) -> MutexGuard<'_, HttpRequestData> {
        self.request.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn into_request(self) -> HttpRequestData {
        self.request.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn invalid() -> VariableValue {
        user_data_value(Self::new(Method::GET, String::new()), HTTP_REQUEST_ID)
    }

    fn value(self) -> VariableValue {
        user_data_value(self, HTTP_REQUEST_ID)
    }
}

#[derive(Clone, Debug, Default)]
pub struct PplHttpResponse {
    valid: bool,
    status: u16,
    final_url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    size: usize,
}

impl PplHttpResponse {
    fn invalid() -> VariableValue {
        Self::default().value()
    }

    fn value(self) -> VariableValue {
        user_data_value(self, HTTP_RESPONSE_ID)
    }

    fn content_type(&self) -> String {
        self.headers.get("content-type").cloned().unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
struct HttpFailure {
    code: i32,
    message: String,
}

impl HttpFailure {
    fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    fn error(self) -> PplError {
        PplError::new(ERR_KIND_NET, self.code, self.message)
    }
}

#[derive(Default)]
pub struct PplHttpService {
    board_limiter: tokio::sync::Mutex<Option<(usize, Arc<tokio::sync::Semaphore>)>>,
    node_limiters: tokio::sync::Mutex<HashMap<usize, (usize, Arc<tokio::sync::Semaphore>)>>,
    clients: tokio::sync::Mutex<HashMap<String, CachedClient>>,
}

struct CachedClient {
    addresses: Vec<SocketAddr>,
    connect_timeout_seconds: u64,
    max_header_bytes: u32,
    last_used: u64,
    client: reqwest::Client,
}

const MAX_CACHED_HTTP_CLIENTS: usize = 128;
const MAX_IDLE_CONNECTIONS_PER_HOST: usize = 4;
const HTTP_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

impl PplHttpService {
    async fn board_permit(&self, maximum: usize) -> Result<tokio::sync::OwnedSemaphorePermit, HttpFailure> {
        let maximum = maximum.max(1);
        let semaphore = {
            let mut limiter = self.board_limiter.lock().await;
            match limiter.as_ref() {
                Some((current, semaphore)) if *current == maximum => semaphore.clone(),
                _ => {
                    let semaphore = Arc::new(tokio::sync::Semaphore::new(maximum));
                    *limiter = Some((maximum, semaphore.clone()));
                    semaphore
                }
            }
        };
        semaphore
            .acquire_owned()
            .await
            .map_err(|_| HttpFailure::new(ERR_UNAVAILABLE, "HTTP service is unavailable"))
    }

    async fn node_permit(&self, node: usize, maximum: usize) -> Result<tokio::sync::OwnedSemaphorePermit, HttpFailure> {
        let maximum = maximum.max(1);
        let semaphore = {
            let mut limiters = self.node_limiters.lock().await;
            match limiters.get(&node) {
                Some((current, semaphore)) if *current == maximum => semaphore.clone(),
                _ => {
                    let semaphore = Arc::new(tokio::sync::Semaphore::new(maximum));
                    limiters.insert(node, (maximum, semaphore.clone()));
                    semaphore
                }
            }
        };
        semaphore
            .acquire_owned()
            .await
            .map_err(|_| HttpFailure::new(ERR_UNAVAILABLE, "HTTP service is unavailable"))
    }

    async fn client(&self, options: &PplHttpOptions, host: &str, addresses: &[SocketAddr]) -> Result<reqwest::Client, HttpFailure> {
        let port = addresses.first().map_or(0, SocketAddr::port);
        let key = format!("{}:{port}", host.to_ascii_lowercase());
        let connect_timeout_seconds = options.connect_timeout_seconds.max(1);
        let max_header_bytes = options.max_header_bytes.min(u32::MAX as usize) as u32;
        let mut clients = self.clients.lock().await;
        let next_use = clients.values().map(|client| client.last_used).max().unwrap_or(0).saturating_add(1);
        if let Some(cached) = clients.get_mut(&key)
            && cached.addresses == addresses
            && cached.connect_timeout_seconds == connect_timeout_seconds
            && cached.max_header_bytes == max_header_bytes
        {
            cached.last_used = next_use;
            return Ok(cached.client.clone());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(connect_timeout_seconds))
            .http2_max_header_list_size(max_header_bytes)
            .pool_max_idle_per_host(MAX_IDLE_CONNECTIONS_PER_HOST)
            .pool_idle_timeout(HTTP_POOL_IDLE_TIMEOUT)
            .resolve_to_addrs(host, addresses)
            .build()
            .map_err(|error| HttpFailure::new(ERR_UNAVAILABLE, format!("cannot create HTTP client: {error}")))?;
        if !clients.contains_key(&key)
            && clients.len() >= MAX_CACHED_HTTP_CLIENTS
            && let Some(oldest) = clients.iter().min_by_key(|(_, client)| client.last_used).map(|(key, _)| key.clone())
        {
            clients.remove(&oldest);
        }
        clients.insert(
            key,
            CachedClient {
                addresses: addresses.to_vec(),
                connect_timeout_seconds,
                max_header_bytes,
                last_used: next_use,
                client: client.clone(),
            },
        );
        Ok(client)
    }

    async fn execute(&self, node: usize, options: &PplHttpOptions, request: HttpRequestData, download: Option<&Path>) -> Result<PplHttpResponse, HttpFailure> {
        if matches!(options.destination_policy, PplHttpDestinationPolicy::Disabled) {
            return Err(HttpFailure::new(ERR_UNAVAILABLE, "PPL HTTP access is disabled"));
        }
        if request.body.len() > options.max_request_bytes {
            return Err(HttpFailure::new(ERR_LIMIT, "HTTP request body exceeds the configured limit"));
        }
        validate_headers(options, &request.headers, "request")?;
        let timeout = Duration::from_secs(options.request_timeout_seconds.max(1));
        tokio::time::timeout(timeout, async {
            let _node_permit = self.node_permit(node, options.max_concurrent_per_node).await?;
            let _board_permit = self.board_permit(options.max_concurrent_requests).await?;
            execute_request(self, options, request, download).await
        })
        .await
        .map_err(|_| HttpFailure::new(ERR_TIMEOUT, "HTTP request timed out"))?
    }
}

async fn execute_request(
    service: &PplHttpService,
    options: &PplHttpOptions,
    mut request: HttpRequestData,
    download: Option<&Path>,
) -> Result<PplHttpResponse, HttpFailure> {
    let mut url = Url::parse(&request.url).map_err(|error| HttpFailure::new(ERR_INVALID, format!("invalid HTTP URL: {error}")))?;
    let mut redirects = 0;

    loop {
        let addresses = validate_destination(options, &url).await?;
        let host = url.host_str().ok_or_else(|| HttpFailure::new(ERR_INVALID, "HTTP URL has no host"))?;
        let client = service.client(options, host, &addresses).await?;
        let mut response = client
            .request(request.method.clone(), url.clone())
            .headers(request.headers.clone())
            .body(request.body.clone())
            .send()
            .await
            .map_err(request_failure)?;

        if is_redirect(response.status()) {
            if redirects >= options.max_redirects {
                return Err(HttpFailure::new(ERR_LIMIT, "HTTP redirect limit exceeded"));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| HttpFailure::new(ERR_FORMAT, "HTTP redirect has no Location header"))?
                .to_str()
                .map_err(|_| HttpFailure::new(ERR_FORMAT, "HTTP redirect Location is not valid text"))?;
            let next = url
                .join(location)
                .map_err(|error| HttpFailure::new(ERR_FORMAT, format!("invalid HTTP redirect: {error}")))?;
            if url.scheme() == "https" && next.scheme() == "http" && !options.allow_http {
                return Err(HttpFailure::new(ERR_DENIED, "HTTP redirect would downgrade HTTPS"));
            }
            if response.status() == StatusCode::SEE_OTHER
                || ((response.status() == StatusCode::MOVED_PERMANENTLY || response.status() == StatusCode::FOUND) && request.method == Method::POST)
            {
                request.method = Method::GET;
                request.body.clear();
            }
            if url.origin() != next.origin() {
                request.headers.clear();
            }
            url = next;
            redirects += 1;
            continue;
        }

        if request.method != Method::HEAD
            && let Some(length) = response.content_length()
            && length > options.max_response_bytes as u64
        {
            return Err(HttpFailure::new(ERR_LIMIT, "HTTP response exceeds the configured limit"));
        }

        let status = response.status();
        validate_headers(options, response.headers(), "response")?;
        let headers = response_headers(response.headers());
        let retain_body = download.is_none() || !status.is_success();
        let mut body = Vec::new();
        let mut size = 0usize;
        let mut output = if download.is_some() && status.is_success() {
            Some(DownloadFile::create(download.expect("download path was checked"))?)
        } else {
            None
        };
        while let Some(chunk) = response.chunk().await.map_err(request_failure)? {
            size = size
                .checked_add(chunk.len())
                .ok_or_else(|| HttpFailure::new(ERR_LIMIT, "HTTP response size overflow"))?;
            if size > options.max_response_bytes {
                return Err(HttpFailure::new(ERR_LIMIT, "HTTP response exceeds the configured limit"));
            }
            if let Some(output) = &mut output {
                output.write(&chunk)?;
            } else {
                body.extend_from_slice(&chunk);
            }
        }
        if let Some(output) = output {
            output.commit()?;
        }
        return Ok(PplHttpResponse {
            valid: true,
            status: status.as_u16(),
            final_url: url.to_string(),
            headers,
            body: retain_body.then_some(body),
            size,
        });
    }
}

async fn validate_destination(options: &PplHttpOptions, url: &Url) -> Result<Vec<SocketAddr>, HttpFailure> {
    match url.scheme() {
        "https" => {}
        "http" if options.allow_http => {}
        "http" => return Err(HttpFailure::new(ERR_DENIED, "plain HTTP is disabled")),
        _ => return Err(HttpFailure::new(ERR_UNSUPPORTED, "only HTTP and HTTPS URLs are supported")),
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(HttpFailure::new(ERR_INVALID, "HTTP URL credentials are not allowed"));
    }
    let host = url.host_str().ok_or_else(|| HttpFailure::new(ERR_INVALID, "HTTP URL has no host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| HttpFailure::new(ERR_INVALID, "HTTP URL has no port"))?;

    if matches!(options.destination_policy, PplHttpDestinationPolicy::Allowlist) {
        let origin = url.origin().ascii_serialization();
        let allowed = options
            .allowed_origins
            .iter()
            .any(|entry| normalize_ppl_http_origin(entry).is_ok_and(|allowed| allowed.eq_ignore_ascii_case(&origin)));
        if !allowed {
            return Err(HttpFailure::new(ERR_DENIED, format!("HTTP origin is not allowed: {origin}")));
        }
    }

    let mut addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| HttpFailure::new(ERR_IO, format!("cannot resolve HTTP host: {error}")))?
        .collect();
    addresses.sort_unstable();
    addresses.dedup();
    if addresses.is_empty() {
        return Err(HttpFailure::new(ERR_IO, "HTTP host resolved to no addresses"));
    }
    if matches!(options.destination_policy, PplHttpDestinationPolicy::Public) && addresses.iter().any(|address| !is_public_address(address.ip())) {
        return Err(HttpFailure::new(ERR_DENIED, "HTTP host resolves to a non-public address"));
    }
    Ok(addresses)
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    !(address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || address.is_multicast()
        || address.is_broadcast()
        || address.is_documentation()
        || address.is_unspecified()
        || octets[0] == 0
        || octets[0] >= 240
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19)))
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    if let Some(mapped) = address.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    let segments = address.segments();
    let globally_routable_unicast = (segments[0] & 0xe000) == 0x2000;
    globally_routable_unicast
        && !(address.is_loopback()
            || address.is_unspecified()
            || address.is_multicast()
            || (segments[0] & 0xfe00) == 0xfc00
            || (segments[0] & 0xffc0) == 0xfe80
            || (segments[0] == 0x2001 && segments[1] <= 0x01ff)
            || (segments[0] == 0x2001 && segments[1] == 0x0db8)
            || segments[0] == 0x2002
            || (segments[0] & 0xfff0) == 0x3ff0)
}

fn is_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER | StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT
    )
}

fn request_failure(error: reqwest::Error) -> HttpFailure {
    if error.is_timeout() {
        HttpFailure::new(ERR_TIMEOUT, format!("HTTP request timed out: {error}"))
    } else {
        HttpFailure::new(ERR_IO, format!("HTTP request failed: {error}"))
    }
}

fn response_headers(headers: &HeaderMap) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (name, value) in headers {
        if let Ok(value) = value.to_str() {
            result
                .entry(name.as_str().to_ascii_lowercase())
                .and_modify(|current: &mut String| {
                    current.push_str(", ");
                    current.push_str(value);
                })
                .or_insert_with(|| value.to_string());
        }
    }
    result
}

fn validate_headers(options: &PplHttpOptions, headers: &HeaderMap, direction: &str) -> Result<(), HttpFailure> {
    let bytes = headers
        .iter()
        .try_fold(0usize, |total, (name, value)| total.checked_add(name.as_str().len() + value.as_bytes().len()))
        .ok_or_else(|| HttpFailure::new(ERR_LIMIT, format!("HTTP {direction} headers are too large")))?;
    if headers.len() > options.max_headers || bytes > options.max_header_bytes {
        return Err(HttpFailure::new(ERR_LIMIT, format!("HTTP {direction} headers exceed the configured limit")));
    }
    Ok(())
}

struct DownloadFile {
    file: tempfile::NamedTempFile,
    destination: PathBuf,
}

impl DownloadFile {
    fn create(destination: &Path) -> Result<Self, HttpFailure> {
        let parent = destination.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent).map_err(file_failure)?;
        let file = tempfile::NamedTempFile::new_in(parent).map_err(file_failure)?;
        Ok(Self {
            file,
            destination: destination.to_path_buf(),
        })
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), HttpFailure> {
        self.file.write_all(bytes).map_err(file_failure)
    }

    fn commit(mut self) -> Result<(), HttpFailure> {
        self.file.flush().map_err(file_failure)?;
        self.file.as_file().sync_all().map_err(file_failure)?;
        replace_file(self.file, &self.destination).map_err(file_failure)
    }
}

fn file_failure(error: std::io::Error) -> HttpFailure {
    HttpFailure::new(ERR_IO, format!("cannot write HTTP download: {error}"))
}

#[cfg(not(windows))]
fn replace_file(source: tempfile::NamedTempFile, destination: &Path) -> std::io::Result<()> {
    source.persist(destination).map(|_| ()).map_err(|error| error.error)
}

#[cfg(windows)]
fn replace_file(source: tempfile::NamedTempFile, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW};

    let source = source.into_temp_path().keep().map_err(|error| error.error)?;
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(source_wide.as_ptr(), destination.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
        let error = std::io::Error::last_os_error();
        let _ = std::fs::remove_file(source);
        return Err(error);
    }
    Ok(())
}

fn method_from_value(value: i32) -> Result<Method, HttpFailure> {
    match value {
        0 => Ok(Method::GET),
        1 => Ok(Method::HEAD),
        2 => Ok(Method::POST),
        _ => Err(HttpFailure::new(ERR_UNSUPPORTED, format!("unsupported HTTP method {value}"))),
    }
}

fn method_value(method: &Method) -> i32 {
    if *method == Method::HEAD {
        1
    } else if *method == Method::POST {
        2
    } else {
        0
    }
}

fn forbidden_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host" | "connection" | "content-length" | "transfer-encoding" | "proxy-authorization" | "proxy-authenticate" | "upgrade" | "te" | "trailer"
    )
}

async fn send(vm: &mut crate::vm::VirtualMachine<'_>, request: HttpRequestData, download: Option<PathBuf>) -> VariableValue {
    let node = vm.icy_board_state.node;
    let (options, service) = {
        let board = vm.icy_board_state.get_board().await;
        (board.config.ppl_http.clone(), board.ppl_http_service.clone())
    };
    match service.execute(node, &options, request, download.as_deref()).await {
        Ok(response) => {
            vm.operation_succeeded();
            response.value()
        }
        Err(failure) => {
            vm.set_error(failure.error());
            PplHttpResponse::invalid()
        }
    }
}

impl UserData for PplHttp {
    const TYPE_NAME: &'static str = "Http";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplHttp::value);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_named_static_function(
            GET.clone(),
            vec![("url", VariableType::UnboundedString)],
            VariableType::UserData(HTTP_RESPONSE_ID as u8),
        );
        registry.add_named_static_function(
            NEW.clone(),
            vec![("method", VariableType::UserData(HTTP_METHOD_ENUM_ID)), ("url", VariableType::UnboundedString)],
            VariableType::UserData(HTTP_REQUEST_ID as u8),
        );
        registry.add_named_static_function(
            DOWNLOAD.clone(),
            vec![("url", VariableType::UnboundedString), ("file", VariableType::UnboundedString)],
            VariableType::UserData(HTTP_RESPONSE_ID as u8),
        );
        registry.add_named_static_function_with(
            URL_ENCODE.clone(),
            vec![("text", VariableType::UnboundedString), ("form", VariableType::Boolean)],
            1,
            VariableType::UnboundedString,
        );
        registry.add_named_static_function_with(
            URL_DECODE.clone(),
            vec![("text", VariableType::UnboundedString), ("form", VariableType::Boolean)],
            1,
            VariableType::UnboundedString,
        );
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplHttp {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        Err(format!("Unknown HTTP property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("HTTP property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        let url = arguments.first().map(VariableValue::as_string).unwrap_or_default();
        if *name == *GET {
            return Ok(send(vm, PplHttpRequest::new(Method::GET, url).into_request(), None).await);
        }
        if *name == *NEW {
            let method = match method_from_value(arguments.first().map_or(0, VariableValue::as_int)) {
                Ok(method) => method,
                Err(failure) => {
                    vm.set_error(failure.error());
                    return Ok(PplHttpRequest::invalid());
                }
            };
            let url = arguments.get(1).map(VariableValue::as_string).unwrap_or_default();
            vm.operation_succeeded();
            return Ok(PplHttpRequest::new(method, url).value());
        }
        if *name == *DOWNLOAD {
            let file = arguments.get(1).map(VariableValue::as_string).unwrap_or_default();
            let path = vm.resolve_file(&file).await;
            return Ok(send(vm, PplHttpRequest::new(Method::GET, url).into_request(), Some(path)).await);
        }
        if *name == *URL_ENCODE || *name == *URL_DECODE {
            let text = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            let form = arguments.get(1).is_none_or(VariableValue::as_bool);
            vm.operation_succeeded();
            let encoded = if *name == *URL_ENCODE {
                url_encode(&text, form)
            } else {
                url_decode(&text, form)
            };
            return Ok(VariableValue::new_unbounded_string(encoded));
        }
        Err(format!("Unknown HTTP function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown HTTP method {name}").into())
    }
}

impl UserData for PplHttpRequest {
    const TYPE_NAME: &'static str = "HttpRequest";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplHttpRequest::invalid);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(URL.clone(), VariableType::UnboundedString, false);
        registry.add_property(METHOD.clone(), VariableType::UserData(HTTP_METHOD_ENUM_ID), false);
        registry.add_named_function(
            SET_HEADER.clone(),
            vec![("name", VariableType::UnboundedString), ("value", VariableType::UnboundedString)],
            VariableType::Boolean,
        );
        registry.add_named_function_with(
            SET_TEXT.clone(),
            vec![("text", VariableType::UnboundedString), ("contentType", VariableType::UnboundedString)],
            1,
            VariableType::Boolean,
        );
        registry.add_named_function(
            SET_FORM.clone(),
            vec![("name", VariableType::UnboundedString), ("value", VariableType::UnboundedString)],
            VariableType::Boolean,
        );
        registry.add_function(SEND.clone(), Vec::new(), VariableType::UserData(HTTP_RESPONSE_ID as u8));
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplHttpRequest {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let request = self.request();
        if *name == *URL {
            return Ok(VariableValue::new_unbounded_string(request.url.clone()));
        }
        if *name == *METHOD {
            return Ok(VariableValue::new_int(method_value(&request.method)));
        }
        Err(format!("Unknown HTTPREQUEST property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("HTTPREQUEST property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *SET_HEADER {
            let header_name = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            let header_value = arguments.get(1).map(VariableValue::as_string).unwrap_or_default();
            let parsed_name = HeaderName::from_bytes(header_name.as_bytes());
            let parsed_value = HeaderValue::from_str(&header_value);
            match (parsed_name, parsed_value) {
                (Ok(header_name), Ok(header_value)) if !forbidden_header(&header_name) => {
                    let mut request = self.request();
                    request.headers.insert(header_name, header_value);
                    vm.operation_succeeded();
                    return Ok(VariableValue::new_bool(true));
                }
                _ => {
                    vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "invalid or restricted HTTP header"));
                    return Ok(VariableValue::new_bool(false));
                }
            }
        }
        if *name == *SET_TEXT {
            let bodyless = {
                let request = self.request();
                request.method == Method::GET || request.method == Method::HEAD
            };
            if bodyless {
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "HTTP GET and HEAD requests cannot carry a body"));
                return Ok(VariableValue::new_bool(false));
            }
            let text = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            let content_type = arguments
                .get(1)
                .map(VariableValue::as_string)
                .unwrap_or_else(|| "text/plain; charset=utf-8".to_string());
            let Ok(content_type) = HeaderValue::from_str(&content_type) else {
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "invalid HTTP content type"));
                return Ok(VariableValue::new_bool(false));
            };
            let mut request = self.request();
            request.body = text.into_bytes();
            request.headers.insert(reqwest::header::CONTENT_TYPE, content_type);
            vm.operation_succeeded();
            return Ok(VariableValue::new_bool(true));
        }
        if *name == *SET_FORM {
            let field = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            let value = arguments.get(1).map(VariableValue::as_string).unwrap_or_default();
            let mut request = self.request();
            if request.method == Method::GET || request.method == Method::HEAD {
                drop(request);
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "HTTP GET and HEAD requests cannot carry a body"));
                return Ok(VariableValue::new_bool(false));
            }
            let is_form = request
                .headers
                .get(reqwest::header::CONTENT_TYPE)
                .is_some_and(|value| value.as_bytes().starts_with(FORM_CONTENT_TYPE.as_bytes()));
            let existing = if is_form {
                String::from_utf8(request.body.clone()).ok()
            } else if request.body.is_empty() {
                Some(String::new())
            } else {
                None
            };
            let Some(existing) = existing else {
                drop(request);
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "request already carries a body that is not form data"));
                return Ok(VariableValue::new_bool(false));
            };
            request.body = form_urlencoded::Serializer::new(existing).append_pair(&field, &value).finish().into_bytes();
            request
                .headers
                .insert(reqwest::header::CONTENT_TYPE, HeaderValue::from_static(FORM_CONTENT_TYPE));
            vm.operation_succeeded();
            return Ok(VariableValue::new_bool(true));
        }
        if *name == *SEND {
            let request = self.request().clone();
            return Ok(send(vm, request, None).await);
        }
        Err(format!("Unknown HTTPREQUEST function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown HTTPREQUEST method {name}").into())
    }
}

impl UserData for PplHttpResponse {
    const TYPE_NAME: &'static str = "HttpResponse";
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = Some(PplHttpResponse::invalid);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        registry.add_property(OK.clone(), VariableType::Boolean, false);
        registry.add_property(STATUS.clone(), VariableType::Integer, false);
        registry.add_property(FINAL_URL.clone(), VariableType::UnboundedString, false);
        registry.add_property(SIZE.clone(), VariableType::Long, false);
        registry.add_property(CONTENT_TYPE.clone(), VariableType::UnboundedString, false);
        registry.add_function(TEXT.clone(), Vec::new(), VariableType::UnboundedString);
        registry.add_named_function(HEADER.clone(), vec![("name", VariableType::UnboundedString)], VariableType::UnboundedString);
        registry.add_named_function(SAVE.clone(), vec![("file", VariableType::UnboundedString)], VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplHttpResponse {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        if *name == *VALID {
            return Ok(VariableValue::new_bool(self.valid));
        }
        if *name == *OK {
            return Ok(VariableValue::new_bool(self.valid && (200..300).contains(&self.status)));
        }
        if *name == *STATUS {
            return Ok(VariableValue::new_int(i32::from(self.status)));
        }
        if *name == *FINAL_URL {
            return Ok(VariableValue::new_unbounded_string(self.final_url.clone()));
        }
        if *name == *SIZE {
            return Ok(VariableValue::new_long(self.size as i64));
        }
        if *name == *CONTENT_TYPE {
            return Ok(VariableValue::new_unbounded_string(self.content_type()));
        }
        Err(format!("Unknown HTTPRESPONSE property {name}").into())
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _value: VariableValue) -> crate::Res<()> {
        Err(format!("HTTPRESPONSE property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *TEXT {
            if !self.valid {
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "invalid HTTP response has no body"));
                return Ok(VariableValue::new_unbounded_string(String::new()));
            }
            let Some(body) = &self.body else {
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "HTTP response body was not retained"));
                return Ok(VariableValue::new_unbounded_string(String::new()));
            };
            return match String::from_utf8(body.clone()) {
                Ok(text) => {
                    vm.operation_succeeded();
                    Ok(VariableValue::new_unbounded_string(text))
                }
                Err(_) => {
                    vm.set_error(PplError::new(ERR_KIND_NET, ERR_FORMAT, "HTTP response body is not UTF-8"));
                    Ok(VariableValue::new_unbounded_string(String::new()))
                }
            };
        }
        if *name == *HEADER {
            let header = arguments.first().map(VariableValue::as_string).unwrap_or_default().to_ascii_lowercase();
            vm.operation_succeeded();
            return Ok(VariableValue::new_unbounded_string(self.headers.get(&header).cloned().unwrap_or_default()));
        }
        if *name == *SAVE {
            if !self.valid {
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "invalid HTTP response has no body to save"));
                return Ok(VariableValue::new_bool(false));
            }
            let Some(body) = &self.body else {
                vm.set_error(PplError::new(ERR_KIND_NET, ERR_INVALID, "HTTP response body was not retained"));
                return Ok(VariableValue::new_bool(false));
            };
            let file = arguments.first().map(VariableValue::as_string).unwrap_or_default();
            let path = vm.resolve_file(&file).await;
            match DownloadFile::create(&path).and_then(|mut output| {
                output.write(body)?;
                output.commit()
            }) {
                Ok(()) => {
                    vm.operation_succeeded();
                    return Ok(VariableValue::new_bool(true));
                }
                Err(failure) => {
                    vm.set_error(failure.error());
                    return Ok(VariableValue::new_bool(false));
                }
            }
        }
        Err(format!("Unknown HTTPRESPONSE function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown HTTPRESPONSE method {name}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_policy_rejects_non_public_addresses() {
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.1.1",
            "100.64.0.1",
            "192.0.2.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
        ] {
            assert!(!is_public_address(address.parse().unwrap()), "{address}");
        }
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(is_public_address(address.parse().unwrap()), "{address}");
        }
    }

    #[tokio::test]
    async fn a_resolved_target_reuses_its_client() {
        let service = PplHttpService::default();
        let options = PplHttpOptions::default();
        let addresses = vec!["127.0.0.1:443".parse().unwrap()];

        service.client(&options, "example.com", &addresses).await.unwrap();
        service.client(&options, "example.com", &addresses).await.unwrap();

        assert_eq!(service.clients.lock().await.len(), 1);
        service.client(&options, "example.com", &["127.0.0.2:443".parse().unwrap()]).await.unwrap();
        assert_eq!(service.clients.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn the_client_cache_evicts_its_least_recently_used_entry() {
        let service = PplHttpService::default();
        let options = PplHttpOptions::default();
        for index in 0..=MAX_CACHED_HTTP_CLIENTS {
            let host = format!("host-{index}.example");
            service.client(&options, &host, &["127.0.0.1:443".parse().unwrap()]).await.unwrap();
        }
        let clients = service.clients.lock().await;
        assert_eq!(clients.len(), MAX_CACHED_HTTP_CLIENTS);
        assert!(!clients.contains_key("host-0.example:443"));
        assert!(clients.contains_key(&format!("host-{MAX_CACHED_HTTP_CLIENTS}.example:443")));
    }

    #[tokio::test]
    async fn each_node_has_its_own_concurrency_limit() {
        let service = PplHttpService::default();
        let first = service.node_permit(1, 1).await.unwrap();
        assert!(service.node_limiters.lock().await[&1].1.clone().try_acquire_owned().is_err());
        let second = service.node_permit(2, 1).await.unwrap();
        assert!(service.node_limiters.lock().await[&2].1.clone().try_acquire_owned().is_err());
        drop(second);
        drop(first);
        assert!(service.node_limiters.lock().await[&1].1.clone().try_acquire_owned().is_ok());
    }

    #[test]
    fn allowlist_entries_are_exact_origins() {
        assert_eq!(normalize_ppl_http_origin("https://example.com"), Ok("https://example.com".to_string()));
        assert_eq!(
            normalize_ppl_http_origin("https://example.com:8443/"),
            Ok("https://example.com:8443".to_string())
        );
        assert!(normalize_ppl_http_origin("https://example.com/path").is_err());
        assert!(normalize_ppl_http_origin("https://example.com/?token=x").is_err());
        assert!(normalize_ppl_http_origin("file:///tmp/data").is_err());
    }
}
