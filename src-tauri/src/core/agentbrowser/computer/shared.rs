use super::*;

pub const TEXT_LIMIT: usize = 6000;

pub(super) const NAVIGATION_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) const ACTION_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) const START_TIMEOUT: Duration = Duration::from_secs(20);

pub const MAX_LIVE: usize = 8;

pub const IDLE_SECS: f64 = 30.0 * 60.0;

pub const FRAME_MAX_BYTES: usize = 4 * 1024 * 1024;

pub const FRAME_KEEP_DAYS: f64 = 30.0;

pub fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// What the live view is sent.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Screen {
    /// A JPEG, base64, as Chrome encoded it.
    Frame { data: String, width: u32, height: u32 },
    Stopped { reason: String },
}

/// The page as a reader sees it.
#[derive(Debug, Clone, Serialize)]
pub struct PageText {
    pub url: String,
    pub title: String,
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub persona: String,
    pub running: bool,
    pub url: String,
    pub title: String,
    pub control: ControlView,
    pub has_profile: bool,
    pub viewers: usize,
    pub last_used: f64,
}

/// The refs a snapshot minted, and which generation they belong to.
#[derive(Debug, Default)]
pub(super) struct Refs {
    pub(super) generation: u64,
    /// The generation the last snapshot was taken at; 0 when none has been.
    pub(super) snapshot: u64,
    pub(super) elements: Vec<Element>,
    pub(super) url: String,
    pub(super) title: String,
}

impl Refs {
    pub(super) fn bump(&mut self) {
        self.generation += 1;
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct Page {
    pub(super) target: String,
    pub(super) session: String,
    /// The session a screencast is running on, if any.
    pub(super) casting: Option<String>,
}

/// What the event loop and the actions share.
pub(super) struct Shared {
    pub(super) persona: String,
    pub(super) data_dir: PathBuf,
    pub(super) refs: Mutex<Refs>,
    pub(super) control: Mutex<Control>,
    pub(super) page: Mutex<Page>,
    pub(super) screen: broadcast::Sender<Screen>,
    pub(super) viewers: AtomicUsize,
    pub(super) last_used: Mutex<f64>,
    pub(super) pid: AtomicU32,
    /// What the page reported on its own since the last look (`note_of`), newest last.
    pub(super) notes: Mutex<VecDeque<String>>,
}

/// The most page notes kept between looks; past it the oldest go.
pub(super) const NOTES_CAP: usize = 60;

pub(super) struct Running {
    pub(super) child: tokio::process::Child,
    pub(super) cdp: Arc<Cdp>,
    pub(super) proxy: proxy::Proxy,
    pub(super) events: tokio::task::JoinHandle<()>,
}

pub struct Computer {
    pub(super) shared: Arc<Shared>,
    /// Held for the length of every action, and around starting and stopping — so two
    /// calls can never interleave on one page.
    pub(super) running: tokio::sync::Mutex<Option<Running>>,
}
