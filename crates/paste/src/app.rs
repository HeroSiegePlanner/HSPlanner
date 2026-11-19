//! Routing and the rules for accepting a paste, independent of the HTTP server.
use std::{
    borrow::Cow,
    collections::HashMap,
    io,
    net::IpAddr,
    sync::{Mutex, MutexGuard, PoisonError},
    time::{Duration, Instant},
};

use hsplanner_build::{
    BuildSnapshot, codec,
    loadout::{LoadoutKind, Loadouts},
};
use hsplanner_engine::calc::{
    data,
    types::{Inventory, TreeSocketContent},
};
use serde_json::json;

use crate::html::BUILD_PATH;
use crate::{
    assets,
    limit::Limiter,
    page::{self, Failure, Selection},
    store::Store,
};

/// Longest pasted text: a share code, or a link that carries one.
const MAX_INPUT: usize = codec::MAX_CODE_LENGTH + 1024;
/// Longest request body; a form post can triple the text with percent escapes.
pub const MAX_BODY: usize = 3 * MAX_INPUT;
pub const SHARES_PER_ADDRESS: u32 = 30;
const SHARES_TOTAL: u32 = 600;
/// Every paste attempt is decoded, so failed attempts have their own, larger budget.
pub const ATTEMPTS_PER_ADDRESS: u32 = 120;
const ATTEMPTS_TOTAL: u32 = 6000;
const LIMIT_WINDOW: Duration = Duration::from_secs(3600);
const CACHED_PAGES: usize = 256;
/// A build that takes longer than this to draw is refused instead of stored.
const RENDER_BUDGET: Duration = Duration::from_secs(5);
const MAX_ITEM_MODS: usize = 64;
const MAX_CUSTOM_STATS: usize = 200;
const MAX_RANKED: usize = 4000;

const CACHE_NONE: &str = "no-store";
const CACHE_PAGE: &str = "public, max-age=60";
const CACHE_FILE: &str = "public, max-age=604800";
const CACHE_FOREVER: &str = "public, max-age=31536000, immutable";

pub struct Request<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub address: IpAddr,
}

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Cow<'static, [u8]>,
    pub location: Option<String>,
    pub cache_control: &'static str,
}

impl Response {
    fn new(status: u16, content_type: &'static str, body: impl Into<Cow<'static, [u8]>>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
            location: None,
            cache_control: CACHE_NONE,
        }
    }

    fn html(status: u16, html: String) -> Self {
        Self::new(status, "text/html; charset=utf-8", html.into_bytes())
    }

    pub fn text(status: u16, text: &str) -> Self {
        Self::new(
            status,
            "text/plain; charset=utf-8",
            text.as_bytes().to_vec(),
        )
    }

    fn not_found() -> Self {
        Self::html(
            404,
            page::message(
                "No build at this address",
                "The link may be mistyped, or the build was never shared here.",
            ),
        )
    }

    fn cached(mut self, cache_control: &'static str) -> Self {
        self.cache_control = cache_control;
        self
    }
}

pub struct App {
    store: Store,
    public_url: String,
    attempts: Mutex<Limiter>,
    shares: Mutex<Limiter>,
    // ponytail: rendered pages, dropped wholesale when full and rendered once per
    // concurrent first visitor; an LRU with per-id locking if either shows up in load.
    pages: Mutex<HashMap<(String, Selection), Vec<u8>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The limiter key: IPv6 visitors own a whole /64, so they are counted by it.
fn limit_key(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(address) => {
            let [a, b, c, d, ..] = address.segments();
            IpAddr::V6([a, b, c, d, 0, 0, 0, 0].into())
        }
    }
}

fn overloaded_items(inventory: &Inventory) -> bool {
    let slots = data::game_config().slots.iter().flatten().count();
    inventory.len() > slots
        || inventory
            .values()
            .any(|item| item.affixes.len() + item.forged_mods.len() > MAX_ITEM_MODS)
}

/// Whether a build holds more than the game could, which only serves to waste the server's time.
fn oversized(snapshot: &BuildSnapshot) -> bool {
    let ranked = [
        snapshot.allocated_tree_nodes.len(),
        snapshot.allocated_ether_nodes.len(),
        snapshot.skill_ranks.len(),
        snapshot.subskill_ranks.len(),
        snapshot.tree_socketed.len(),
    ];
    let heavy_socket = snapshot.tree_socketed.values().any(|content| {
        matches!(content, TreeSocketContent::Uncut { affixes } if affixes.len() > MAX_ITEM_MODS)
    });
    overloaded_items(&snapshot.inventory)
        || overloaded_items(&snapshot.merc_inventory)
        || snapshot.custom_stats.len() > MAX_CUSTOM_STATS
        || ranked.into_iter().any(|count| count > MAX_RANKED)
        || heavy_socket
}

/// Whether any loadout a visitor could switch to is oversized, not just the shared ones.
fn oversized_loadout(snapshot: &BuildSnapshot, loadouts: &Loadouts) -> bool {
    let (mut loadouts, mut snapshot) = (loadouts.clone(), snapshot.clone());
    for kind in LoadoutKind::ALL {
        let ids: Vec<String> = loadouts
            .entries(kind)
            .into_iter()
            .map(|(id, _)| id.to_owned())
            .collect();
        for id in ids {
            // The size limits are per part, so one pass over each kind covers every combination.
            if loadouts.select(kind, &id).is_err() {
                return true;
            }
            loadouts.apply_active(&mut snapshot);
            if oversized(&snapshot) {
                return true;
            }
        }
    }
    false
}

impl App {
    pub fn new(store: Store, public_url: String) -> Self {
        let now = Instant::now();
        let attempts = Limiter::new(LIMIT_WINDOW, ATTEMPTS_PER_ADDRESS, ATTEMPTS_TOTAL, now);
        let shares = Limiter::new(LIMIT_WINDOW, SHARES_PER_ADDRESS, SHARES_TOTAL, now);
        Self {
            store,
            public_url,
            attempts: Mutex::new(attempts),
            shares: Mutex::new(shares),
            pages: Mutex::new(HashMap::new()),
        }
    }

    pub fn handle(&self, request: &Request) -> Response {
        let target = request.path.split('#').next().unwrap_or_default();
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        match (request.method, path) {
            ("POST", "/") => self.share_form(request),
            ("POST", "/api/pastes") => self.share_api(request),
            ("POST", _) => Response::not_found(),
            ("GET" | "HEAD", _) => self.read(path, query, request.address),
            _ => Response::text(405, "Method not allowed"),
        }
    }

    fn read(&self, path: &str, query: &str, address: IpAddr) -> Response {
        if path == "/" {
            return Response::html(200, page::home(None, ""));
        }
        if path == "/healthz" {
            return Response::text(200, "ok");
        }
        if let Some(name) = path.strip_prefix("/static/") {
            return assets::fixed(name).map_or_else(Response::not_found, |bytes| {
                Response::new(200, assets::content_type(name), bytes).cached(CACHE_FILE)
            });
        }
        if let Some(name) = path.strip_prefix("/assets/") {
            return assets::game(name).map_or_else(Response::not_found, |bytes| {
                Response::new(200, assets::content_type(name), bytes).cached(CACHE_FILE)
            });
        }
        let Some(rest) = path.strip_prefix(BUILD_PATH) else {
            return Response::not_found();
        };
        let (id, view) = match rest.split_once('/') {
            Some((id, view)) => (id, Some(view)),
            None => (rest, None),
        };
        let code = match self.store.get(id) {
            Ok(Some(code)) => code,
            Ok(None) => return Response::not_found(),
            Err(error) => {
                eprintln!("could not read paste {id}: {error}");
                return Response::html(
                    500,
                    page::message("This build could not be loaded", "Try again in a moment."),
                );
            }
        };
        match view {
            None => match Selection::parse(query) {
                Some(selection) => self.page(id, &code, selection, address),
                None => Response::not_found(),
            },
            Some("raw") => Response::text(200, &code).cached(CACHE_FOREVER),
            Some(_) => Response::not_found(),
        }
    }

    fn page(&self, id: &str, code: &str, selection: Selection, address: IpAddr) -> Response {
        let key = (id.to_owned(), selection);
        let cached = lock(&self.pages).get(&key).cloned();
        if let Some(html) = cached {
            return Response::new(200, "text/html; charset=utf-8", html).cached(CACHE_PAGE);
        }
        // Only the shared loadouts were timed when the build was stored, so drawing others is rationed.
        if !selection.is_default()
            && !lock(&self.attempts).allow(limit_key(address), Instant::now())
        {
            let detail = "Too many loadouts were opened from here. Try again in an hour.";
            return Response::html(429, page::message("Slow down", detail));
        }
        match page::build(id, code, &self.public_url, selection) {
            Ok(html) => {
                let html = html.into_bytes();
                let mut pages = lock(&self.pages);
                if pages.len() >= CACHED_PAGES {
                    pages.clear();
                }
                pages.insert(key, html.clone());
                Response::new(200, "text/html; charset=utf-8", html).cached(CACHE_PAGE)
            }
            Err(Failure::NoSuchLoadout) => Response::not_found(),
            Err(Failure::Unreadable(error)) => {
                Response::html(500, page::message("This build cannot be shown", &error))
            }
        }
    }

    /// Validates pasted text and stores its share code; errors carry an HTTP status.
    fn share(&self, input: &str, address: IpAddr) -> Result<String, (u16, String)> {
        let refuse = |status: u16, message: &str| Err((status, message.to_owned()));
        if input.len() > MAX_INPUT {
            return refuse(413, "This build code is too large.");
        }
        let (address, now) = (limit_key(address), Instant::now());
        if !lock(&self.attempts).allow(address, now) {
            return refuse(429, "Too many attempts from here. Try again in an hour.");
        }
        let code = codec::parse_input(input).map_err(|error| (400, error))?;
        let (snapshot, _, loadouts) =
            codec::decode_loadouts(&code).map_err(|error| (400, error))?;
        if oversized(&snapshot) || oversized_loadout(&snapshot, &loadouts) {
            return refuse(422, "This build holds more than a character can carry.");
        }
        // Drawing the page once proves every later visit is affordable.
        let started = Instant::now();
        match page::build("preview", &code, "", Selection::default()) {
            Ok(_) => {}
            Err(Failure::Unreadable(error)) => return Err((400, error)),
            Err(Failure::NoSuchLoadout) => return refuse(400, "This build has no such loadout."),
        }
        if started.elapsed() > RENDER_BUDGET {
            return refuse(422, "This build takes too long to calculate.");
        }
        if !lock(&self.shares).allow(address, now) {
            return refuse(
                429,
                "Too many builds were shared from here. Try again in an hour.",
            );
        }
        self.store.create(&code).or_else(|error| {
            if error.kind() == io::ErrorKind::StorageFull {
                return refuse(507, "This site has no room for new builds right now.");
            }
            eprintln!("could not store a paste: {error}");
            refuse(500, "The build could not be saved. Try again later.")
        })
    }

    fn share_form(&self, request: &Request) -> Response {
        if request.body.len() > MAX_BODY {
            let error = "This build code is too large.";
            return Response::html(413, page::home(Some(error), ""));
        }
        let body = String::from_utf8_lossy(request.body);
        let input = form_field(&body, "code").unwrap_or_default();
        match self.share(&input, request.address) {
            Ok(id) => {
                let mut response = Response::text(303, "");
                response.location = Some(format!("{BUILD_PATH}{id}"));
                response
            }
            Err((status, error)) => {
                let draft = if status == 413 { "" } else { input.as_str() };
                Response::html(status, page::home(Some(&error), draft))
            }
        }
    }

    fn share_api(&self, request: &Request) -> Response {
        let input = String::from_utf8_lossy(request.body);
        let (status, reply) = match self.share(&input, request.address) {
            Ok(id) => {
                let url = format!("{}{BUILD_PATH}{id}", self.public_url);
                let data = json!({ "id": id, "url": url });
                (201, json!({ "success": true, "data": data, "error": null }))
            }
            Err((status, error)) => (
                status,
                json!({ "success": false, "data": null, "error": error }),
            ),
        };
        Response::new(
            status,
            "application/json; charset=utf-8",
            reply.to_string().into_bytes(),
        )
    }
}

fn form_field(body: &str, name: &str) -> Option<String> {
    body.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| form_decode(value))
    })
}

fn form_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = |offset: usize| {
            bytes
                .get(index + offset)
                .and_then(|byte| char::from(*byte).to_digit(16))
        };
        match (bytes[index], hex(1), hex(2)) {
            (b'%', Some(high), Some(low)) => {
                decoded.push((high * 16 + low) as u8);
                index += 2;
            }
            (b'+', ..) => decoded.push(b' '),
            (byte, ..) => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
