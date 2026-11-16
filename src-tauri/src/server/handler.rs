//! The commands the browser can run: the app's own (src/lib.rs `generate_handler!`), minus the ones about its
//! windows. `tests/web_commands.rs` fails when the UI calls one that is neither here nor in `NOT_IN_WEB`.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;

use super::glue::{Reply, WebCommand};
use super::Server;
use crate::commands;

type Run = for<'a> fn(&'a Server, Value) -> Reply<'a>;

/// Commands about the desktop the server runs on, which nobody is looking at.
pub const NOT_IN_WEB: &[&str] = &[
    "quit_app",
    // Opens an editor on the server.
    "open_in_editor",
    // The app's own development and bug reports.
    "log_frontend",
    "dev_launch_target",
    "report_issue",
];

macro_rules! web_handler {
    ($($command:path),* $(,)?) => {
        static COMMANDS: LazyLock<HashMap<&'static str, Run>> = LazyLock::new(|| {
            HashMap::from([$((<$command as WebCommand>::NAME, <$command as WebCommand>::run as Run)),*])
        });
    };
}

pub fn find(name: &str) -> Option<Run> {
    COMMANDS.get(name).copied()
}

pub fn names() -> impl Iterator<Item = &'static str> {
    COMMANDS.keys().copied()
}

web_handler![
    super::web_commands::take_open_requests,
    commands::terminal::terminal_status,
    commands::terminal::terminal_set_allowed,
    commands::terminal::terminal_connect,
    commands::terminal::terminal_disconnect,
    commands::terminal::terminal_install_cli,
    commands::terminal::terminal_uninstall_cli,
    commands::repo::open_repo,
    commands::repo::quick_open_roots,
    commands::repo::list_dir_suggestions,
    commands::repo::resolve_repo_root,
    commands::repo::clone_repo,
    commands::repo::recent_repos,
    commands::repo::project_data,
    commands::repo::clear_project_data,
    commands::repo::reset_all_data,
    commands::repo::repo_remote_urls,
    commands::repo::watch_repo,
    commands::repo::unwatch_repo,
    commands::repo::list_commits,
    commands::repo::list_branch_commits,
    commands::repo::list_branches,
    commands::repo::list_worktrees,
    commands::tasks::list_tasks,
    commands::tasks::main_checkout,
    commands::tasks::create_task,
    commands::tasks::set_task_state,
    commands::tasks::rename_task,
    commands::tasks::finish_task_run,
    commands::tasks::archive_task,
    commands::tasks::notify_task,
    commands::tasks::set_task_badge,
    commands::tasks::mark_task_reviewed,
    commands::tasks::task_config,
    commands::tasks::run_task_setup,
    commands::tasks::task_base_status,
    commands::tasks::update_task_from_base,
    commands::tasks::ship_task_pr,
    commands::tasks::merge_task_locally,
    commands::tasks::restore_task,
    commands::tasks::list_archived_tasks,
    commands::tasks::task_storage,
    commands::tasks::refresh_task_prs,
    commands::tasks::adopt_worktree,
    commands::repo::git_status,
    commands::repo::repo_overview,
    commands::repo::get_setting,
    commands::repo::set_setting,
    commands::diff::resolve_ref,
    commands::diff::get_diff,
    commands::diff::get_file_patch,
    commands::diff::get_file_versions,
    commands::diff::diff_fingerprint,
    commands::diff::revert_file,
    commands::diff::revert_hunk,
    commands::diff::apply_suggestion,
    commands::diff::get_diff_ignore_rules,
    commands::diff::set_diff_ignore_rules,
    commands::files::list_tree,
    commands::files::read_file,
    commands::files::read_file_base64,
    commands::files::search_code,
    commands::comments::get_session,
    commands::comments::list_threads,
    commands::comments::list_repo_threads,
    commands::comments::create_thread,
    commands::comments::add_reply,
    commands::comments::edit_comment,
    commands::comments::delete_comment,
    commands::comments::delete_thread,
    commands::comments::restore_thread,
    commands::comments::keep_thread_on_branch,
    commands::comments::delete_all_threads,
    commands::comments::set_thread_status,
    commands::comments::list_viewed,
    commands::comments::get_guide,
    commands::comments::delete_guide,
    commands::comments::set_viewed,
    commands::comments::viewed_changes,
    commands::comments::get_pending_review,
    commands::comments::start_review,
    commands::comments::get_review,
    commands::comments::list_reviews,
    commands::comments::submit_review,
    commands::comments::discard_review,
    commands::agents::list_agents,
    commands::agents::agent_models,
    commands::agents::start_chat,
    commands::agents::list_chats,
    commands::agents::get_chat_messages,
    commands::agents::send_prompt,
    commands::agents::cancel_prompt,
    commands::agents::respond_permission,
    commands::agents::delete_chat,
    commands::github::github_auth_status,
    commands::github::github_use_gh,
    commands::github::github_set_token,
    commands::github::github_device_start,
    commands::github::github_device_poll,
    commands::github::github_logout,
    commands::github::git_fetch,
    commands::github::git_pull,
    commands::github::git_push,
    commands::github::git_commit_all,
    commands::github::github_post_comment,
    commands::github::find_pr,
    commands::github::list_prs,
    commands::github::github_pr_inbox,
    commands::github::checkout_pr,
    commands::github::update_pr_branch,
    commands::github::git_stash_push,
    commands::github::git_stash_restore,
    commands::github::git_checkout,
    commands::github::git_clone,
    commands::github::push_review,
    commands::github::pull_review,
    commands::github::github_pushable_threads,
    commands::github::github_review_candidates,
    commands::github::github_reply,
    commands::github::github_set_resolved,
];
