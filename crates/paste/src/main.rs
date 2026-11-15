//! hspaste: a pastebin for HSPlanner build codes, small enough for a Raspberry Pi.
mod app;
mod assets;
mod equipment;
mod gear;
mod html;
mod limit;
mod numbers;
mod page;
mod panels;
mod store;
mod tree;

use std::{
    env,
    io::Read,
    net::{IpAddr, Ipv4Addr},
    panic::{self, AssertUnwindSafe},
    process,
    sync::Arc,
    thread,
};

use app::{App, MAX_BODY, Request, Response};
use tiny_http::{Header, Method, Server};

const DEFAULT_ADDRESS: &str = "127.0.0.1:8787";
const DEFAULT_DATA_DIR: &str = "paste-data";
const DEFAULT_MAX_BYTES: &str = "2147483648";
// ponytail: a worker reads one request at a time, so stalled uploads can occupy all of
// them; keep the buffering reverse proxy in front, or move to an async server.
const WORKERS: usize = 8;
/// Room for the engine's and the HTML parser's recursion on the largest accepted build.
const WORKER_STACK: usize = 16 * 1024 * 1024;
const SECURITY_HEADERS: [(&str, &str); 4] = [
    (
        "Content-Security-Policy",
        "default-src 'none'; img-src 'self'; style-src 'self'; font-src 'self'; \
script-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
    ),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    ("X-Frame-Options", "DENY"),
];

fn setting(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn fail(message: String) -> ! {
    eprintln!("hspaste: {message}");
    process::exit(1);
}

fn main() {
    let address = setting("HSPASTE_ADDR", DEFAULT_ADDRESS);
    let data_dir = setting("HSPASTE_DATA_DIR", DEFAULT_DATA_DIR);
    let max_bytes = setting("HSPASTE_MAX_BYTES", DEFAULT_MAX_BYTES);
    let max_bytes: u64 = max_bytes
        .parse()
        .unwrap_or_else(|_| fail(format!("HSPASTE_MAX_BYTES is not a number: {max_bytes}")));
    let public_url = setting("HSPASTE_PUBLIC_URL", "")
        .trim_end_matches('/')
        .to_owned();
    let trust_proxy = env::var_os("HSPASTE_TRUST_PROXY").is_some();
    let store = store::Store::open(&data_dir, max_bytes)
        .unwrap_or_else(|error| fail(format!("cannot use data directory {data_dir}: {error}")));
    let server = Server::http(&address)
        .unwrap_or_else(|error| fail(format!("cannot listen on {address}: {error}")));
    // Load the game data now, so the first visitor does not wait for it.
    hsplanner_engine::calc::data::data();
    let app = Arc::new(App::new(store, public_url));
    let server = Arc::new(server);
    eprintln!("hspaste: listening on http://{address}, pastes in {data_dir}");
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (app, server) = (Arc::clone(&app), Arc::clone(&server));
            let work = move || {
                for request in server.incoming_requests() {
                    serve(&app, request, trust_proxy);
                }
            };
            thread::Builder::new()
                .stack_size(WORKER_STACK)
                .spawn(work)
                .unwrap_or_else(|error| fail(format!("cannot start a worker: {error}")))
        })
        .collect();
    for worker in workers {
        let _ = worker.join();
    }
}

/// The body of a paste request; other requests have theirs ignored unread.
fn read_body(request: &mut tiny_http::Request) -> Result<Vec<u8>, Response> {
    let mut body = Vec::new();
    if *request.method() != Method::Post {
        return Ok(body);
    }
    let too_large = || Response::text(413, "This build code is too large.");
    if request
        .body_length()
        .is_some_and(|length| length > MAX_BODY)
    {
        return Err(too_large());
    }
    let limit = MAX_BODY as u64 + 1;
    match request.as_reader().take(limit).read_to_end(&mut body) {
        Ok(_) if body.len() > MAX_BODY => Err(too_large()),
        Ok(_) => Ok(body),
        Err(error) => {
            eprintln!("hspaste: could not read a request body: {error}");
            Err(Response::text(400, "The request could not be read."))
        }
    }
}

fn serve(app: &App, mut request: tiny_http::Request, trust_proxy: bool) {
    let method = request.method().as_str().to_owned();
    let path = request.url().to_owned();
    let address = client_address(&request, trust_proxy);
    let response = match read_body(&mut request) {
        Err(refusal) => refusal,
        Ok(body) => {
            let handled = panic::catch_unwind(AssertUnwindSafe(|| {
                app.handle(&Request {
                    method: &method,
                    path: &path,
                    body: &body,
                    address,
                })
            }));
            handled.unwrap_or_else(|_| Response::text(500, "Something went wrong."))
        }
    };
    let mut headers = vec![
        ("Content-Type", response.content_type.to_owned()),
        ("Cache-Control", response.cache_control.to_owned()),
    ];
    headers.extend(
        SECURITY_HEADERS
            .iter()
            .map(|(name, value)| (*name, (*value).to_owned())),
    );
    headers.extend(response.location.map(|location| ("Location", location)));
    let mut reply = tiny_http::Response::from_data(response.body.into_owned())
        .with_status_code(response.status);
    for (name, value) in headers {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            reply.add_header(header);
        }
    }
    if let Err(error) = request.respond(reply) {
        eprintln!("hspaste: could not send a response: {error}");
    }
}

/// The visitor's address. A reverse proxy on this machine is believed about the
/// address it forwards for; anyone else connecting directly is not.
fn client_address(request: &tiny_http::Request, trust_proxy: bool) -> IpAddr {
    let peer = request
        .remote_addr()
        .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |address| address.ip());
    if !trust_proxy || !peer.is_loopback() {
        return peer;
    }
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("X-Forwarded-For"))
        .and_then(|header| {
            header
                .value
                .as_str()
                .rsplit(',')
                .next()?
                .trim()
                .parse()
                .ok()
        })
        .unwrap_or(peer)
}
