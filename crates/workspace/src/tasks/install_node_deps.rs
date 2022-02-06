use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_error::map_io_to_fs_error;
use moon_logger::debug;
use std::fs;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn install_node_deps(workspace: Arc<RwLock<Workspace>>) -> Result<(), WorkspaceError> {
    let workspace = workspace.read().await;
    let toolchain = &workspace.toolchain;
    let manager = toolchain.get_package_manager();
    let mut cache = workspace.cache.workspace_state().await?;

    // Get the last modified time of the root lockfile
    let lockfile = workspace.root.join(manager.get_lockfile_name());
    let mut last_modified = 0;

    if lockfile.exists() {
        let lockfile_metadata =
            fs::metadata(&lockfile).map_err(|e| map_io_to_fs_error(e, lockfile.clone()))?;

        last_modified = cache.to_millis(
            lockfile_metadata
                .modified()
                .map_err(|e| map_io_to_fs_error(e, lockfile.clone()))?,
        );
    }

    // Install deps if the lockfile has been modified
    // since the last time dependencies were installed!
    if last_modified == 0 || last_modified > cache.item.last_node_install_time {
        debug!(
            target: "moon:task-runner:install-node-deps",
            "Installing Node.js dependencies",
        );

        manager.install_dependencies(toolchain).await?;

        if let Some(node_config) = &workspace.config.node {
            if node_config.dedupe_on_install.unwrap_or(true) {
                debug!(
                    target: "moon:task-runner:install-node-deps",
                    "Dedupeing dependencies",
                );

                manager.dedupe_dependencies(toolchain).await?;
            }
        }

        // Update the cache with the timestamp
        cache.item.last_node_install_time = cache.now_millis();
        cache.save().await?;
    } else {
        debug!(
            target: "moon:task-runner:install-node-deps",
            "Lockfile has not changed since last install, skipping Node.js dependencies",
        );
    }

    Ok(())
}