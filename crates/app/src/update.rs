use crate::release_notes;
use anyhow::Context as _;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use futures::{AsyncReadExt, StreamExt};
use gpui_kit::http_client::{
    AsyncBody, HttpClient, HttpRequestExt, RedirectPolicy, Request, Response, http::header,
};
use gpui_kit::*;
use hsplanner_ui::i18n::trf;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

pub const REPO: &str = "HeroSiegePlanner/HSPlanner";
pub const USER_AGENT: &str = concat!("HSPlanner/", env!("CARGO_PKG_VERSION"));
// cargo-packager names installers after `productName` in packaging/packager.json;
// SHA256SUMS identifies releases produced by our packaging pipeline.
const ASSET_PREFIX: &str = "HSPlanner_";
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
const CHUNK: usize = 64 * 1024;
const UNSIZED_REPORT_STEP: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub version: Version,
    pub page: String,
    pub installer: Option<Asset>,
    pub notes: Vec<release_notes::Section>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Download {
    pub received: u64,
    pub total: Option<u64>,
}

impl Download {
    pub fn percent(&self) -> Option<f32> {
        let total = self.total.filter(|total| *total > 0)?;
        Some((self.received as f32 / total as f32 * 100.).min(100.))
    }

    fn worth_reporting(&self, previous: &Download) -> bool {
        match (self.percent(), previous.percent()) {
            (Some(now), Some(before)) => now.floor() > before.floor(),
            _ => self.received / UNSIZED_REPORT_STEP > previous.received / UNSIZED_REPORT_STEP,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Update),
    Downloading {
        update: Update,
        progress: Download,
    },
    Installing(Update),
    Failed {
        failure: Failure,
        update: Option<Update>,
    },
}

impl State {
    pub fn update(&self) -> Option<&Update> {
        match self {
            State::Available(update)
            | State::Downloading { update, .. }
            | State::Installing(update) => Some(update),
            State::Failed { update, .. } => update.as_ref(),
            _ => None,
        }
    }

    pub fn busy(&self) -> bool {
        matches!(
            self,
            State::Checking | State::Downloading { .. } | State::Installing(_)
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    key: &'static str,
    detail: String,
}

impl Failure {
    pub fn message(&self) -> String {
        trf(self.key, &[("error", self.detail.clone())])
    }
}

enum Step {
    Received(Download),
    Installing,
}

pub struct Installed;

pub struct Updater {
    pub state: State,
}

impl EventEmitter<Installed> for Updater {}

impl Updater {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self { state: State::Idle };
        this.check(true, cx);
        this
    }

    pub fn check(&mut self, silent: bool, cx: &mut Context<Self>) {
        if self.state.busy() {
            return;
        }
        self.state = State::Checking;
        cx.notify();
        let http = cx.http_client();
        let task = cx.background_spawn(async move { fetch_latest(http).await });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.state = match result {
                    Ok(release) => match pick(release, &current_version(), std::env::consts::OS) {
                        Some(update) => State::Available(update),
                        None => State::UpToDate,
                    },
                    Err(error) => {
                        log::warn!("Update check failed: {error:#}");
                        if silent {
                            State::Idle
                        } else {
                            State::Failed {
                                failure: Failure {
                                    key: "update.check_failed",
                                    detail: format!("{error:#}"),
                                },
                                update: None,
                            }
                        }
                    }
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Re-runs whatever failed: the install when an update is known, otherwise the check.
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        let State::Failed { update, .. } = &self.state else {
            return;
        };
        match update.clone() {
            Some(update) => {
                self.state = State::Available(update);
                self.install(cx);
            }
            None => {
                self.state = State::Idle;
                self.check(false, cx);
            }
        }
    }

    pub fn install(&mut self, cx: &mut Context<Self>) {
        let State::Available(update) = self.state.clone() else {
            return;
        };
        let Some(asset) = update.installer.clone() else {
            cx.open_url(&update.page);
            return;
        };
        self.state = State::Downloading {
            update,
            progress: Download::default(),
        };
        cx.notify();
        let http = cx.http_client();
        let (steps, mut received) = unbounded();
        let task = cx.background_spawn(async move {
            let path = download(http, &asset, &steps).await?;
            let _ = steps.unbounded_send(Step::Installing);
            install(&path)
        });
        cx.spawn(async move |this, cx| {
            while let Some(step) = received.next().await {
                let advanced = this.update(cx, |this, cx| {
                    this.state = advance(std::mem::take(&mut this.state), step);
                    cx.notify();
                });
                if advanced.is_err() {
                    return;
                }
            }
            let result = task.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => cx.emit(Installed),
                Err(error) => {
                    log::error!("Update install failed: {error:#}");
                    this.state = State::Failed {
                        failure: Failure {
                            key: "update.install_failed",
                            detail: format!("{error:#}"),
                        },
                        update: this.state.update().cloned(),
                    };
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

fn advance(state: State, step: Step) -> State {
    match (state, step) {
        (State::Downloading { update, .. }, Step::Received(progress)) => {
            State::Downloading { update, progress }
        }
        (State::Downloading { update, .. }, Step::Installing) => State::Installing(update),
        (state, _) => state,
    }
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("valid crate version")
}

fn pick(release: Release, current: &Version, os: &str) -> Option<Update> {
    let version = Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
    if version <= *current
        || !release
            .assets
            .iter()
            .any(|asset| asset.name == "SHA256SUMS")
        || !release.assets.iter().any(|asset| is_package(&asset.name))
    {
        return None;
    }
    let extension = match os {
        "macos" => Some(".dmg"),
        "windows" => Some(".exe"),
        _ => None,
    };
    let installer = extension.and_then(|extension| {
        release
            .assets
            .into_iter()
            .find(|asset| is_package(&asset.name) && asset.name.ends_with(extension))
    });
    Some(Update {
        version,
        page: release.html_url,
        installer,
        notes: release_notes::parse(release.body.as_deref().unwrap_or_default()),
    })
}

// Linux and Windows packages are published in lowercase (`hsplanner_…`).
fn is_package(name: &str) -> bool {
    name.get(..ASSET_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(ASSET_PREFIX))
}

fn feed_url() -> String {
    // A local release JSON lets debug builds rehearse the whole update without publishing.
    std::env::var("HSPLANNER_UPDATE_FEED")
        .ok()
        .filter(|_| cfg!(debug_assertions))
        .unwrap_or_else(|| format!("https://api.github.com/repos/{REPO}/releases/latest"))
}

async fn fetch_latest(http: Arc<dyn HttpClient>) -> anyhow::Result<Release> {
    let request = Request::get(feed_url())
        .header("Accept", "application/vnd.github+json")
        .follow_redirects(RedirectPolicy::FollowAll)
        .timeout(CHECK_TIMEOUT)
        .body(AsyncBody::default())?;
    let body = read(http.send(request).await?).await?;
    serde_json::from_slice(&body).context("unexpected release JSON")
}

async fn download(
    http: Arc<dyn HttpClient>,
    asset: &Asset,
    steps: &UnboundedSender<Step>,
) -> anyhow::Result<PathBuf> {
    let expected = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .context(
            "GitHub published no checksum for this installer; download it from the release page",
        )?;
    anyhow::ensure!(
        Path::new(&asset.name).file_name() == Some(asset.name.as_ref()),
        "unexpected installer name {}",
        asset.name
    );
    let request = Request::get(&asset.browser_download_url)
        .follow_redirects(RedirectPolicy::FollowAll)
        .timeout(DOWNLOAD_TIMEOUT)
        .body(AsyncBody::default())?;
    let mut response = http.send(request).await?;
    anyhow::ensure!(
        response.status().is_success(),
        "GitHub answered {}",
        response.status()
    );
    let mut progress = Download {
        received: 0,
        total: response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|length| length.to_str().ok()?.parse().ok()),
    };
    let directory = std::env::temp_dir().join("hsplanner-update");
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("creating {}", directory.display()))?;
    let path = directory.join(&asset.name);
    let mut file =
        std::fs::File::create(&path).with_context(|| format!("writing {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; CHUNK];
    let mut reported = progress;
    loop {
        let read = response.body_mut().read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .with_context(|| format!("writing {}", path.display()))?;
        progress.received += read as u64;
        if progress.worth_reporting(&reported) {
            reported = progress;
            let _ = steps.unbounded_send(Step::Received(progress));
        }
    }
    file.flush()?;
    if !digest_matches(hasher, expected) {
        let _ = std::fs::remove_file(&path);
        anyhow::bail!("checksum mismatch for {}", asset.name);
    }
    Ok(path)
}

async fn read(mut response: Response<AsyncBody>) -> anyhow::Result<Vec<u8>> {
    let status = response.status();
    let mut body = Vec::new();
    response.body_mut().read_to_end(&mut body).await?;
    anyhow::ensure!(status.is_success(), "GitHub answered {status}");
    Ok(body)
}

fn digest_matches(hasher: Sha256, expected_hex: &str) -> bool {
    format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected_hex)
}

#[cfg(target_os = "macos")]
fn install(dmg: &Path) -> anyhow::Result<()> {
    let bundle = std::env::current_exe().ok().and_then(|exe| bundle_of(&exe));
    let Some(bundle) = bundle else {
        run("open", &[dmg.as_os_str()])?;
        anyhow::bail!(
            "not running from an app bundle; the disk image was opened for manual installation"
        );
    };
    let mount = std::env::temp_dir().join("hsplanner-update-mount");
    let detach = || {
        run(
            "hdiutil",
            &["detach".as_ref(), mount.as_os_str(), "-force".as_ref()],
        )
    };
    // An interrupted earlier update may still hold the mount point.
    let _ = detach();
    run(
        "hdiutil",
        &[
            "attach".as_ref(),
            "-nobrowse".as_ref(),
            "-readonly".as_ref(),
            "-mountpoint".as_ref(),
            mount.as_os_str(),
            dmg.as_os_str(),
        ],
    )?;
    let replaced = replace_bundle(&mount, &bundle);
    let _ = detach();
    if let Err(error) = replaced {
        let _ = run("open", &[dmg.as_os_str()]);
        return Err(error.context("the disk image was opened for manual installation"));
    }
    // Relaunch once this process has exited, so `open` starts the new bundle instead of
    // re-activating the old instance.
    std::process::Command::new("sh")
        .arg("-c")
        .arg("while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; open \"$0\"")
        .arg(&bundle)
        .arg(std::process::id().to_string())
        .spawn()
        .context("scheduling the relaunch")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn replace_bundle(mount: &Path, bundle: &Path) -> anyhow::Result<()> {
    let fresh = std::fs::read_dir(mount)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .context("no application in the disk image")?;
    let staged = bundle.with_extension("app.update");
    let old = bundle.with_extension("app.old");
    let _ = std::fs::remove_dir_all(&staged);
    let _ = std::fs::remove_dir_all(&old);
    run("ditto", &[fresh.as_os_str(), staged.as_os_str()])?;
    std::fs::rename(bundle, &old).with_context(|| format!("replacing {}", bundle.display()))?;
    if let Err(error) = std::fs::rename(&staged, bundle) {
        let _ = std::fs::rename(&old, bundle);
        return Err(error).with_context(|| format!("replacing {}", bundle.display()));
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

#[cfg(target_os = "macos")]
fn run(program: &str, args: &[&std::ffi::OsStr]) -> anyhow::Result<()> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("running {program}"))?;
    anyhow::ensure!(
        output.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let bundle = exe.ancestors().nth(3)?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

#[cfg(target_os = "windows")]
fn install(installer: &Path) -> anyhow::Result<()> {
    std::process::Command::new(installer)
        .spawn()
        .context("starting the installer")?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn install(_: &Path) -> anyhow::Result<()> {
    anyhow::bail!("in-app installation is not available on this platform")
}

#[cfg(test)]
mod tests {
    // Explicit imports: the crate's `gpui_kit::*` glob would shadow `#[test]` with `gpui::test`.
    use super::{
        Asset, Download, Failure, Release, State, Step, Update, Version, advance, bundle_of,
        digest_matches, pick,
    };
    use sha2::{Digest, Sha256};
    use std::path::Path;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.into(),
            browser_download_url: format!("https://x/{name}"),
            digest: None,
        }
    }

    fn release(tag: &str, names: &[&str]) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: "https://x/release".into(),
            body: None,
            assets: names.iter().map(|name| asset(name)).collect(),
        }
    }

    fn update() -> Update {
        Update {
            version: Version::new(1, 2, 0),
            page: "https://x/release".into(),
            installer: None,
            notes: Vec::new(),
        }
    }

    #[test]
    fn newer_native_release_yields_platform_installer() {
        let current = Version::new(1, 1, 0);
        let release = release(
            "v1.2.0",
            &[
                "HSPlanner_1.2.0_aarch64.dmg",
                "HSPlanner_1.2.0_x64-setup.exe",
                "SHA256SUMS",
            ],
        );
        let update = pick(release, &current, "macos").unwrap();
        assert_eq!(update.version, Version::new(1, 2, 0));
        assert_eq!(
            update.installer.unwrap().name,
            "HSPlanner_1.2.0_aarch64.dmg"
        );
    }

    #[test]
    fn windows_picks_setup_exe_and_linux_gets_page_only() {
        let names = [
            "HSPlanner_1.2.0_aarch64.dmg",
            "HSPlanner_1.2.0_x64-setup.exe",
            "HSPlanner_1.2.0_amd64.deb",
            "SHA256SUMS",
        ];
        let windows = pick(release("v1.2.0", &names), &Version::new(1, 1, 0), "windows").unwrap();
        assert_eq!(
            windows.installer.unwrap().name,
            "HSPlanner_1.2.0_x64-setup.exe"
        );
        let linux = pick(release("v1.2.0", &names), &Version::new(1, 1, 0), "linux").unwrap();
        assert_eq!(linux.installer, None);
        assert_eq!(linux.page, "https://x/release");
    }

    #[test]
    fn published_lowercase_package_names_still_match() {
        let names = [
            "HSPlanner_1.2.0_aarch64.dmg",
            "hsplanner_1.2.0_x64-setup.exe",
            "hsplanner_1.2.0_amd64.deb",
            "SHA256SUMS",
        ];
        let windows = pick(release("v1.2.0", &names), &Version::new(1, 1, 0), "windows").unwrap();
        assert_eq!(
            windows.installer.unwrap().name,
            "hsplanner_1.2.0_x64-setup.exe"
        );
        let only_lowercase = ["hsplanner_1.2.0_x64-setup.exe", "SHA256SUMS"];
        assert!(
            pick(
                release("v1.2.0", &only_lowercase),
                &Version::new(1, 1, 0),
                "windows"
            )
            .is_some()
        );
    }

    #[test]
    fn same_or_older_version_is_not_an_update() {
        let names = ["HSPlanner_1.1.0_aarch64.dmg", "SHA256SUMS"];
        assert!(pick(release("v1.1.0", &names), &Version::new(1, 1, 0), "macos").is_none());
        assert!(pick(release("v1.0.9", &names), &Version::new(1, 1, 0), "macos").is_none());
        assert!(pick(release("nightly", &names), &Version::new(1, 1, 0), "macos").is_none());
    }

    #[test]
    fn prerelease_suffix_orders_before_the_final_version() {
        let names = ["HSPlanner_1.2.0_aarch64.dmg", "SHA256SUMS"];
        let current = Version::parse("1.2.0-beta.1").unwrap();
        assert!(pick(release("v1.2.0", &names), &current, "macos").is_some());
        assert!(pick(release("v1.2.0-beta.1", &names), &current, "macos").is_none());
    }

    #[test]
    fn legacy_tauri_release_is_ignored_even_when_newer() {
        let release = release(
            "v9.0.0",
            &[
                "HSPlanner_9.0.0_aarch64.dmg",
                "HSPlanner_9.0.0_x64-setup.exe",
                "latest.json",
            ],
        );
        assert!(pick(release, &Version::new(1, 1, 0), "macos").is_none());
    }

    #[test]
    fn release_body_becomes_sectioned_notes() {
        let mut release = release("v1.2.0", &["HSPlanner_1.2.0_aarch64.dmg", "SHA256SUMS"]);
        release.body = Some("## Feature\n\n- Faster `trees`\n".into());
        let notes = pick(release, &Version::new(1, 1, 0), "macos")
            .unwrap()
            .notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title.as_deref(), Some("Feature"));
        assert_eq!(notes[0].items, ["Faster trees"]);
    }

    #[test]
    fn digest_comparison_is_case_insensitive() {
        let expected = "2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824";
        let hash = |bytes: &[u8]| {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            hasher
        };
        assert!(digest_matches(hash(b"hello"), expected));
        assert!(!digest_matches(hash(b"hello!"), expected));
    }

    #[test]
    fn progress_reports_whole_percents_or_each_mebibyte_when_unknown() {
        let sized = |received| Download {
            received,
            total: Some(1000),
        };
        assert_eq!(sized(505).percent(), Some(50.5));
        assert!(!sized(505).worth_reporting(&sized(500)));
        assert!(sized(510).worth_reporting(&sized(505)));
        assert_eq!(sized(2000).percent(), Some(100.));
        let unknown = |received| Download {
            received,
            total: None,
        };
        assert_eq!(unknown(10).percent(), None);
        assert!(!unknown(1_000_000).worth_reporting(&unknown(0)));
        assert!(unknown(1_048_576).worth_reporting(&unknown(1_000_000)));
    }

    #[test]
    fn steps_move_a_download_forward_and_never_revive_other_states() {
        let downloading = State::Downloading {
            update: update(),
            progress: Download::default(),
        };
        let progress = Download {
            received: 5,
            total: Some(10),
        };
        assert_eq!(
            advance(downloading.clone(), Step::Received(progress)),
            State::Downloading {
                update: update(),
                progress
            }
        );
        assert_eq!(
            advance(downloading, Step::Installing),
            State::Installing(update())
        );
        assert_eq!(advance(State::Idle, Step::Installing), State::Idle);
    }

    #[test]
    fn failed_install_keeps_the_update_for_retry() {
        let failure = Failure {
            key: "update.install_failed",
            detail: "boom".into(),
        };
        let with_update = State::Failed {
            failure: failure.clone(),
            update: Some(update()),
        };
        assert_eq!(with_update.update(), Some(&update()));
        assert!(!with_update.busy());
        let check_failed = State::Failed {
            failure,
            update: None,
        };
        assert_eq!(check_failed.update(), None);
    }

    #[test]
    #[ignore = "talks to api.github.com; run with --ignored"]
    fn live_release_json_parses_and_carries_digests() {
        let http: std::sync::Arc<dyn super::HttpClient> = std::sync::Arc::new(
            reqwest_client::ReqwestClient::user_agent(super::USER_AGENT).unwrap(),
        );
        let release = futures::executor::block_on(super::fetch_latest(http)).unwrap();
        assert!(release.tag_name.starts_with('v'), "{}", release.tag_name);
        assert!(release.html_url.contains("/releases/tag/"));
        assert!(release.assets.iter().all(|asset| {
            asset
                .digest
                .as_deref()
                .is_some_and(|d| d.starts_with("sha256:"))
        }));
    }

    #[test]
    fn bundle_is_three_levels_above_the_executable() {
        let exe = Path::new("/Applications/HSPlanner.app/Contents/MacOS/hsplanner");
        assert_eq!(
            bundle_of(exe).unwrap(),
            Path::new("/Applications/HSPlanner.app")
        );
        assert_eq!(bundle_of(Path::new("/repo/target/release/hsplanner")), None);
    }
}
