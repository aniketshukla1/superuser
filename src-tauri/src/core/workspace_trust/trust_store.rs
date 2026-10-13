use super::*;

/// Where a project declares what it considers routine.
pub const CONFIG_PATH: &str = ".superuser/config.json";

/// How many prefixes one project may declare.
///
/// A cap rather than a policy question: a repo that wants two hundred command
/// prefixes auto-approved is not describing its build, and nobody reviewing the
/// trust prompt would read the list anyway.
pub(super) const MAX_DECLARED: usize = 40;

/// What a project asked for.
#[derive(Debug, Default, Deserialize)]
pub(super) struct RepoConfig {
    #[serde(default, alias = "allowed_commands")]
    pub(super) allow_commands: Vec<String>,
}

/// One row of the Trusted workspaces card. Field names are the GUI's contract.
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceCommandTrust {
    pub workspace: String,
    pub requested_commands: Vec<String>,
    pub trusted: bool,
    /// Does this project actually declare anything? A trusted folder with no
    /// config is not an error — it is the ordinary case after someone removes
    /// their allowances — but the card says so rather than showing an empty list
    /// that looks like a load failure.
    pub required: bool,
    pub exists: bool,
}

/// The user's trust decisions, one JSON array of canonical paths.
pub struct TrustStore {
    pub(super) path: PathBuf,
}

impl TrustStore {
    pub fn load(data_dir: &Path) -> Self {
        Self { path: data_dir.join("workspace-trust.json") }
    }

    /// The one spelling of a folder that gets recorded.
    ///
    /// Resolved through symlinks where possible, so `/tmp/x` and `/private/tmp/x`
    /// cannot be two different trust decisions about one directory. A folder that
    /// no longer exists cannot be resolved, and keeps the spelling it was stored
    /// under — otherwise revoking trust for a deleted project would silently fail
    /// to match the row the card is showing.
    pub fn canonical(path: &str) -> String {
        let p = PathBuf::from(path.trim());
        std::fs::canonicalize(&p).unwrap_or(p).to_string_lossy().to_string()
    }

    pub(super) fn read(&self) -> BTreeSet<String> {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
            .map(|v| v.into_iter().collect())
            .unwrap_or_default()
    }

    pub(super) fn write(&self, set: &BTreeSet<String>) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&set.iter().collect::<Vec<_>>()).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, text).map_err(|e| e.to_string())
    }

    pub fn is_trusted(&self, path: &str) -> bool {
        self.read().contains(&Self::canonical(path))
    }

    pub fn set(&self, path: &str, trusted: bool) -> Result<(), String> {
        let key = Self::canonical(path);
        if key.trim().is_empty() {
            return Err("a workspace path is required".into());
        }
        let mut set = self.read();
        if trusted {
            set.insert(key);
        } else {
            set.remove(&key);
        }
        self.write(&set)
    }

    /// Every trusted folder, with what it currently declares.
    pub fn list(&self) -> Vec<WorkspaceCommandTrust> {
        self.read()
            .into_iter()
            .map(|w| {
                let declared = declared_commands(Path::new(&w));
                WorkspaceCommandTrust {
                    exists: Path::new(&w).is_dir(),
                    required: !declared.is_empty(),
                    requested_commands: declared,
                    trusted: true,
                    workspace: w,
                }
            })
            .collect()
    }

    /// The status of one folder, trusted or not — what the card needs to show a
    /// project the user is looking at right now.
    pub fn status(&self, path: &str) -> WorkspaceCommandTrust {
        let w = Self::canonical(path);
        let declared = declared_commands(Path::new(&w));
        WorkspaceCommandTrust {
            trusted: self.read().contains(&w),
            exists: Path::new(&w).is_dir(),
            required: !declared.is_empty(),
            requested_commands: declared,
            workspace: w,
        }
    }
}

/// What a project declares, whether or not anyone trusts it.
///
/// Reading is always safe — it is a list of strings — and the card has to show
/// what an untrusted project is asking for before the user can sensibly decide.
/// Nothing here consults trust; [`allowances`] is the function that does.
pub fn declared_commands(workspace: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(workspace.join(CONFIG_PATH)) else { return Vec::new() };
    let Ok(cfg) = serde_json::from_str::<RepoConfig>(&text) else { return Vec::new() };
    cfg.allow_commands
        .into_iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .take(MAX_DECLARED)
        .collect()
}

/// The command prefixes this workspace contributes **right now**.
///
/// Empty for an untrusted folder, which is the entire point of the module.
pub fn allowances(data_dir: &Path, workspace: &Path) -> Vec<String> {
    let w = workspace.to_string_lossy().to_string();
    if !TrustStore::load(data_dir).is_trusted(&w) {
        return Vec::new();
    }
    declared_commands(workspace)
}
