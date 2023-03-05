use crate::DENO_DEPS;
use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::{config_cache, LockfileDependencyVersions};
use moon_utils::json::read as read_json;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

config_cache!(DenoLock, DENO_DEPS.lockfile, read_json);

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DenoLock {
    remote: FxHashMap<String, String>,

    #[serde(skip)]
    pub path: PathBuf,
}

#[cached(result)]
pub fn load_lockfile_dependencies(path: PathBuf) -> Result<LockfileDependencyVersions, MoonError> {
    let mut deps: LockfileDependencyVersions = FxHashMap::default();

    if let Some(lockfile) = DenoLock::read(path)? {
        for (key, value) in lockfile.remote {
            deps.insert(key, vec![value]);
        }
    }

    Ok(deps)
}