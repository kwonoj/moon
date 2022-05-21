use crate::action::ActionStatus;
use crate::actions::hashing::create_target_hasher;
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_cache::RunTargetState;
use moon_config::TaskType;
use moon_logger::{color, debug};
use moon_project::{Project, Target, Task};
use moon_terminal::output::{label_run_target, label_run_target_failed};
use moon_toolchain::{get_path_env_var, Executable};
use moon_utils::process::{output_to_string, Command, Output};
use moon_utils::{is_ci, path, string_vec};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const TARGET: &str = "moon:action:run-target";

async fn create_env_vars(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<HashMap<String, String>, WorkspaceError> {
    let mut env_vars = HashMap::new();

    env_vars.insert(
        "MOON_CACHE_DIR".to_owned(),
        path::path_to_string(&workspace.cache.dir)?,
    );
    env_vars.insert("MOON_PROJECT_ID".to_owned(), project.id.clone());
    env_vars.insert(
        "MOON_PROJECT_ROOT".to_owned(),
        path::path_to_string(&project.root)?,
    );
    env_vars.insert("MOON_PROJECT_SOURCE".to_owned(), project.source.clone());
    env_vars.insert("MOON_RUN_TARGET".to_owned(), task.target.clone());
    env_vars.insert(
        "MOON_TOOLCHAIN_DIR".to_owned(),
        path::path_to_string(&workspace.toolchain.dir)?,
    );
    env_vars.insert(
        "MOON_WORKSPACE_ROOT".to_owned(),
        path::path_to_string(&workspace.root)?,
    );
    env_vars.insert(
        "MOON_WORKING_DIR".to_owned(),
        path::path_to_string(&workspace.working_dir)?,
    );

    // Store runtime data on the file system so that downstream commands can utilize it
    let runfile = workspace.cache.create_runfile(&project.id, project).await?;

    env_vars.insert(
        "MOON_PROJECT_RUNFILE".to_owned(),
        path::path_to_string(&runfile.path)?,
    );

    Ok(env_vars)
}

fn create_node_options(task: &Task) -> Vec<String> {
    string_vec![
        // "--inspect", // Enable node inspector
        "--preserve-symlinks",
        "--title",
        &task.target,
        "--unhandled-rejections",
        "throw",
    ]
}

/// Runs a task command through our toolchain's installed Node.js instance.
/// We accomplish this by executing the Node.js binary as a child process,
/// while passing a file path to a package's node module binary (this is the file
/// being executed). We then also pass arguments defined in the task.
/// This would look something like the following:
///
/// ~/.moon/tools/node/1.2.3/bin/node --inspect /path/to/node_modules/.bin/eslint
///     --cache --color --fix --ext .ts,.tsx,.js,.jsx
#[cfg(not(windows))]
fn create_node_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let node = workspace.toolchain.get_node();
    let mut cmd = node.get_bin_path();
    let mut args = vec![];

    match task.command.as_str() {
        "node" => {
            args.extend(create_node_options(task));
        }
        "npm" => {
            cmd = node.get_npm().get_bin_path();
        }
        "pnpm" => {
            cmd = node.get_pnpm().unwrap().get_bin_path();
        }
        "yarn" => {
            cmd = node.get_yarn().unwrap().get_bin_path();
        }
        bin => {
            let bin_path = node.find_package_bin_path(bin, &project.root)?;

            args.extend(create_node_options(task));
            args.push(path::path_to_string(&bin_path)?);
        }
    };

    // Create the command
    let mut command = Command::new(cmd);

    command.args(&args).args(&task.args).envs(&task.env).env(
        "PATH",
        get_path_env_var(node.get_bin_path().parent().unwrap()),
    );

    Ok(command)
}

/// Windows works quite differently than other systems, so we cannot do the above.
/// On Windows, the package binary is a ".cmd" file, which means it needs to run
/// through "cmd.exe" and not "node.exe". Because of this, the order of operations
/// is switched, and "node.exe" is detected through the `PATH` env var.
#[cfg(windows)]
fn create_node_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let node = workspace.toolchain.get_node();

    let cmd = match task.command.as_str() {
        "node" => node.get_bin_path().clone(),
        "npm" => node.get_npm().get_bin_path().clone(),
        "pnpm" => node.get_pnpm().unwrap().get_bin_path().clone(),
        "yarn" => node.get_yarn().unwrap().get_bin_path().clone(),
        bin => node.find_package_bin_path(bin, &project.root)?,
    };

    // Create the command
    let mut command = Command::new(cmd);

    command
        .args(&task.args)
        .envs(&task.env)
        .env(
            "PATH",
            get_path_env_var(node.get_bin_path().parent().unwrap()),
        )
        .env("NODE_OPTIONS", create_node_options(task).join(" "));

    Ok(command)
}

#[cfg(not(windows))]
fn create_system_target_command(task: &Task, _cwd: &Path) -> Command {
    let mut cmd = Command::new(&task.command);
    cmd.args(&task.args).envs(&task.env);
    cmd
}

#[cfg(windows)]
fn create_system_target_command(task: &Task, cwd: &Path) -> Command {
    use moon_utils::process::is_windows_script;

    let mut cmd = Command::new(&task.command);

    for arg in &task.args {
        // cmd.exe requires an absolute path to batch files
        if is_windows_script(arg) {
            cmd.arg(cwd.join(arg));
        } else {
            cmd.arg(arg);
        }
    }

    cmd.envs(&task.env);
    cmd
}

async fn create_target_command(
    workspace: &Workspace,
    project: &Project,
    task: &Task,
) -> Result<Command, WorkspaceError> {
    let working_dir = if task.options.run_from_workspace_root {
        &workspace.root
    } else {
        &project.root
    };

    let mut command = match task.type_of {
        TaskType::Node => create_node_target_command(workspace, project, task)?,
        _ => create_system_target_command(task, working_dir),
    };

    let env_vars = create_env_vars(workspace, project, task).await?;

    command
        .cwd(working_dir)
        .envs(env_vars)
        // We need to handle non-zero's manually
        .no_error_on_failure();

    Ok(command)
}

pub async fn run_target(
    workspace: Arc<RwLock<Workspace>>,
    target_id: &str,
    primary_target: &str,
    passthrough_args: &[String],
) -> Result<ActionStatus, WorkspaceError> {
    debug!(target: TARGET, "Running target {}", color::id(target_id));

    let workspace = workspace.read().await;
    let mut cache = workspace.cache.cache_run_target_state(target_id).await?;

    // Gather the project and task
    let is_primary = primary_target == target_id;
    let (project_id, task_id) = Target::parse(target_id)?.ids()?;
    let project = workspace.projects.load(&project_id)?;
    let task = project.get_task(&task_id)?;

    // Abort early if this build has already been cached/hashed
    let hasher = create_target_hasher(&workspace, &project, task, passthrough_args).await?;
    let hash = hasher.to_hash();

    debug!(
        target: TARGET,
        "Generated hash {} for target {}",
        color::symbol(&hash),
        color::id(target_id)
    );

    if cache.item.hash == hash {
        print_target_label(target_id, "(cached)", cache.item.exit_code != 0);
        print_cache_item(&cache.item);

        return Ok(ActionStatus::Cached);
    }

    // Build the command to run based on the task
    let mut command = create_target_command(&workspace, &project, task).await?;

    if is_primary && !passthrough_args.is_empty() {
        command.args(passthrough_args);
    }

    // Run the command as a child process and capture its output.
    // If the process fails and `retry_count` is greater than 0,
    // attempt the process again in case it passes.
    let attempt_count = task.options.retry_count + 1;
    let mut attempt = 1;
    let stream_output = is_primary || is_ci() && env::var("MOON_TEST").is_err();
    let output;

    loop {
        let attempt_comment = if attempt == 1 {
            String::new()
        } else {
            format!("(attempt {} of {})", attempt, attempt_count)
        };

        let possible_output = if stream_output {
            // Print label *before* output is streamed since it may stay open forever,
            // or it may use ANSI escape codes to alter the terminal.
            print_target_label(target_id, &attempt_comment, false);

            // If this target matches the primary target (the last task to run),
            // then we want to stream the output directly to the parent (inherit mode).
            command.exec_stream_and_capture_output().await
        } else {
            // Otherwise we run the process in the background and write the output
            // once it has completed.
            command.exec_capture_output().await
        };

        match possible_output {
            // zero and non-zero exit codes
            Ok(out) => {
                if stream_output {
                    handle_streamed_output(target_id, &attempt_comment, &out);
                } else {
                    handle_captured_output(target_id, &attempt_comment, &out);
                }

                if out.status.success() {
                    output = out;
                    break;
                } else if attempt >= attempt_count {
                    return Err(WorkspaceError::Moon(command.output_to_error(&out, false)));
                } else {
                    attempt += 1;

                    debug!(
                        target: TARGET,
                        "Target {} failed, running again with attempt {}",
                        color::target(target_id),
                        attempt
                    );
                }
            }
            // process itself failed
            Err(error) => {
                return Err(WorkspaceError::Moon(error));
            }
        }
    }

    // Hard link outputs to the `.moon/cache/out` folder and to the cloud,
    // so that subsequent builds are faster, and any local outputs
    // can be rehydrated easily.
    for output_path in &task.output_paths {
        workspace
            .cache
            .link_task_output_to_out(&hash, &project.root, output_path)
            .await?;
    }

    // Save the new hash
    workspace.cache.save_hash(&hash, &hasher).await?;

    // Write the cache with the result and output
    cache.item.exit_code = output.status.code().unwrap_or(0);
    cache.item.hash = hash;
    cache.item.last_run_time = cache.now_millis();
    cache.item.stderr = output_to_string(&output.stderr);
    cache.item.stdout = output_to_string(&output.stdout);
    cache.save().await?;

    Ok(ActionStatus::Passed)
}

fn print_target_label(target: &str, comment: &str, failed: bool) {
    let mut label = if failed {
        label_run_target_failed(target)
    } else {
        label_run_target(target)
    };

    if !comment.is_empty() {
        label = format!("{} {}", label, color::muted(comment));
    };

    if failed {
        eprintln!("{}", label);
    } else {
        println!("{}", label);
    }
}

fn print_cache_item(item: &RunTargetState) {
    if !item.stderr.is_empty() {
        eprintln!("{}", item.stderr.trim());
        eprintln!();
    }

    if !item.stdout.is_empty() {
        println!("{}", item.stdout.trim());
        println!();
    }
}

fn print_output_std(output: &Output) {
    let stderr = output_to_string(&output.stderr);
    let stdout = output_to_string(&output.stdout);

    if !stderr.is_empty() {
        eprintln!("{}", stderr.trim());
        eprintln!();
    }

    if !stdout.is_empty() {
        println!("{}", stdout.trim());
        println!();
    }
}

// Print label *after* output has been captured, so parallel tasks
// aren't intertwined and the labels align with the output.
fn handle_captured_output(target_id: &str, attempt_comment: &str, output: &Output) {
    print_target_label(target_id, attempt_comment, !output.status.success());
    print_output_std(output);
}

// Only print the label when the process has failed,
// as the actual output has already been streamed to the console.
fn handle_streamed_output(target_id: &str, attempt_comment: &str, output: &Output) {
    if !output.status.success() {
        print_target_label(target_id, attempt_comment, true);
    }
}
