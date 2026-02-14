use git2::{Repository, ResetType, StatusOptions, DiffOptions, RemoteCallbacks, FetchOptions, PushOptions, Cred, StashFlags, ObjectType, Signature};
use std::io::Write;
use std::fs::OpenOptions;
use chrono::{DateTime, Local};

pub mod graph;

pub const GRAPH_COLORS: [&str; 8] = [
    "#4A90E2", // Blue
    "#F5A623", // Orange
    "#D0021B", // Red
    "#F8E71C", // Yellow
    "#7ED321", // Green
    "#9013FE", // Purple
    "#50E3C2", // Cyan
    "#F8A0D8", // Pink
];

pub struct GitHandler;

impl GitHandler {
    pub fn init(path: &str) -> Result<(), String> {
        Repository::init(path).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn clone(url: &str, path: &str) -> Result<(), String> {
        Repository::clone(url, path).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn get_current_branch(path: &str) -> Result<String, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        if head.is_branch() {
            Ok(head.shorthand().unwrap_or("").to_string())
        } else {
            let oid = head.target().ok_or("err-head-target-not-found".to_string())?;
            Ok(oid.to_string()[..7].to_string())
        }
    }

    pub fn get_branches(path: &str) -> Result<Vec<String>, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let branches = repo.branches(Some(git2::BranchType::Local)).map_err(|e| e.message().to_string())?;
        let mut branch_names = Vec::new();
        for branch in branches {
            if let Ok((b, _)) = branch {
                if let Ok(Some(name)) = b.name() { branch_names.push(name.to_string()); }
            }
        }
        Ok(branch_names)
    }

    pub fn get_remote_branches(path: &str) -> Result<Vec<String>, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let branches = repo.branches(Some(git2::BranchType::Remote)).map_err(|e| e.message().to_string())?;
        let mut branch_names = Vec::new();
        for branch in branches {
            if let Ok((b, _)) = branch {
                if let Ok(Some(name)) = b.name() { branch_names.push(name.to_string()); }
            }
        }
        Ok(branch_names)
    }

    pub fn get_tags(path: &str) -> Result<Vec<String>, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let tags = repo.tag_names(None).map_err(|e| e.message().to_string())?;
        Ok(tags.iter().flatten().map(|s| s.to_string()).collect())
    }

    #[allow(dead_code)]
    pub fn create_tag(path: &str, name: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let oid = head.target().ok_or("err-head-target-not-found".to_string())?;
        let target = repo.find_object(oid, Some(ObjectType::Commit)).map_err(|e| e.message().to_string())?;
        repo.tag_lightweight(name, &target, false).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn cherry_pick(path: &str, revision: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(obj.id()).map_err(|e| e.message().to_string())?;
        repo.cherrypick(&commit, None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn revert_commit(path: &str, revision: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(obj.id()).map_err(|e| e.message().to_string())?;
        repo.revert(&commit, None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn reset_hard(path: &str, revision: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        repo.reset(&obj, ResetType::Hard, None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn checkout_commit(path: &str, revision: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        repo.set_head_detached(obj.id()).map_err(|e| e.message().to_string())?;
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.force();
        repo.checkout_head(Some(&mut checkout_opts)).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn add_to_gitignore(path: &str, file: &str) -> Result<(), String> {
        let ignore_path = std::path::Path::new(path).join(".gitignore");
        let mut f = OpenOptions::new().write(true).append(true).create(true).open(ignore_path).map_err(|e| e.to_string())?;
        writeln!(f, "{}", file).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn amend_head(path: &str, new_msg: Option<&str>, new_author: Option<(&str, &str)>) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let oid = head.target().ok_or("err-head-target-not-found".to_string())?;
        let head_commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        
        let tree = head_commit.tree().map_err(|e| e.message().to_string())?;

        if let Some((name, email)) = new_author {
            let new_sig = Signature::now(name, email).map_err(|e| e.message().to_string())?;
            head_commit.amend(Some("HEAD"), Some(&new_sig), Some(&new_sig), None, new_msg, Some(&tree))
                .map_err(|e| e.message().to_string())?;
        } else {
            head_commit.amend(Some("HEAD"), None, None, None, new_msg, Some(&tree))
                .map_err(|e| e.message().to_string())?;
        }
        
        Ok(())
    }

    pub fn squash_parent(path: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let head_ref = repo.head().map_err(|e| e.message().to_string())?;
        let head_commit = head_ref.peel_to_commit().map_err(|e| e.message().to_string())?;

        if head_commit.parent_count() == 0 {
            return Err("No parent to squash into.".to_string());
        }
        let parent_commit = head_commit.parent(0).map_err(|e| e.message().to_string())?;

        repo.reset(parent_commit.as_object(), ResetType::Soft, None)
            .map_err(|e| e.message().to_string())?;

        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let tree_id = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        let grandparents: Vec<_> = parent_commit.parents().collect();
        let grandparents_ref: Vec<&_> = grandparents.iter().collect();
        let message = head_commit.message().unwrap_or_default();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &grandparents_ref).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn save_patch(path: &str, sha: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(sha).map_err(|e| e.message().to_string())?;
        let commit = obj.as_commit().ok_or("Not a commit")?;
        
        let diff = {
            let commit_tree = commit.tree().map_err(|e| e.message().to_string())?;
            if commit.parent_count() > 0 {
                let parent = commit.parent(0).map_err(|e| e.message().to_string())?;
                let parent_tree = parent.tree().map_err(|e| e.message().to_string())?;
                repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)
            } else {
                repo.diff_tree_to_tree(None, Some(&commit_tree), None)
            }
        }.map_err(|e| e.message().to_string())?;

        let mut patch_content = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            patch_content.write_all(&[line.origin() as u8]).is_ok()
                && patch_content.write_all(line.content()).is_ok()
        }).map_err(|e| e.message().to_string())?;

        let filename = format!("{}/{}.patch", path, sha);
        let mut f = OpenOptions::new().write(true).create(true).truncate(true).open(filename).map_err(|e| e.to_string())?;
        f.write_all(&patch_content).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    pub fn apply_patch(path: &str, patch_path: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let content = std::fs::read(patch_path).map_err(|e| e.to_string())?;
        let diff = git2::Diff::from_buffer(&content).map_err(|e| e.message().to_string())?;
        repo.apply(&diff, git2::ApplyLocation::WorkDir, None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn get_commit_details(path: &str, revision: &str) -> Result<(String, String, String, String, String), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(obj.id()).map_err(|e| e.message().to_string())?;
        let author = format!("{} <{}>", commit.author().name().unwrap_or(""), commit.author().email().unwrap_or(""));
        let committer = format!("{} <{}>", commit.committer().name().unwrap_or(""), commit.committer().email().unwrap_or(""));
        let message = commit.message().unwrap_or("").to_string();
        let sha = commit.id().to_string();
        let parents = commit.parent_ids().map(|id| id.to_string()).collect::<Vec<_>>().join(", ");
        Ok((author, committer, message, sha, parents))
    }

    pub fn get_commit_files(path: &str, revision: &str) -> Result<Vec<String>, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(obj.id()).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;
        let parent_tree = if commit.parent_count() > 0 {
            commit.parent(0).ok().and_then(|p| p.tree().ok())
        } else {
            None
        };
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None).map_err(|e| e.message().to_string())?;
        let mut files = Vec::new();
        diff.foreach(&mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                files.push(path.to_string_lossy().to_string());
            }
            true
        }, None, None, None).map_err(|e| e.message().to_string())?;
        Ok(files)
    }

    pub fn get_commit_file_diff(path: &str, revision: &str, file_path: &str) -> Result<String, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let obj = repo.revparse_single(revision).map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(obj.id()).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;
        let parent_tree = if commit.parent_count() > 0 {
            commit.parent(0).ok().and_then(|p| p.tree().ok())
        } else {
            None
        };
        let mut opts = DiffOptions::new();
        opts.pathspec(file_path);
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts)).map_err(|e| e.message().to_string())?;
        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() { '+' => "+", '-' => "-", ' ' => " ", _ => "" };
            diff_text.push_str(prefix);
            diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        }).map_err(|e| e.message().to_string())?;
        Ok(diff_text)
    }

    pub fn checkout_branch(path: &str, branch_name: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let (object, reference) = repo.revparse_ext(branch_name).map_err(|e| e.message().to_string())?;
        repo.checkout_tree(&object, None).map_err(|e| e.message().to_string())?;
        match reference {
            Some(gref) => repo.set_head(gref.name().ok_or("err-invalid-ref-name".to_string())?),
            None => repo.set_head_detached(object.id()),
        }.map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn create_branch(path: &str, name: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let oid = head.target().ok_or("err-head-target-not-found".to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        repo.branch(name, &commit, false).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn delete_branch(path: &str, name: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut branch = repo.find_branch(name, git2::BranchType::Local).map_err(|e| e.message().to_string())?;
        branch.delete().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn discard_changes(path: &str, file: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.path(std::path::Path::new(file));
        checkout_opts.force();
        repo.checkout_index(None, Some(&mut checkout_opts)).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn push(path: &str, user: &str, token: &str, force: bool) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut remote = repo.find_remote("origin").map_err(|e| e.message().to_string())?;
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            Cred::userpass_plaintext(user, token)
        });
        let mut options = PushOptions::new();
        options.remote_callbacks(callbacks);
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let refspec = head.name().ok_or("err-invalid-ref-name".to_string())?;
        let remote_ref = if force { format!("+{}", refspec) } else { refspec.to_string() };
        remote.push(&[format!("{}:{}", remote_ref, refspec)], Some(&mut options)).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn pull(path: &str, user: &str, token: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut remote = repo.find_remote("origin").map_err(|e| e.message().to_string())?;
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            Cred::userpass_plaintext(user, token)
        });
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        let head = repo.head().map_err(|e| e.message().to_string())?;
        let branch_name = head.shorthand().ok_or("err-invalid-branch-name".to_string())?;
        remote.fetch(&[branch_name], Some(&mut fetch_options), None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn fetch(path: &str, user: &str, token: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut remote = repo.find_remote("origin").map_err(|e| e.message().to_string())?;
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            Cred::userpass_plaintext(user, token)
        });
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);
        remote.fetch(&[] as &[&str], Some(&mut fetch_options), None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn get_latest_commits_full(repo_path: &str, limit: usize, all_branches: bool) -> Result<Vec<(String, String, String, String, Vec<String>)>, String> {
        let repo = Repository::open(repo_path).map_err(|e| e.message().to_string())?;
        let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;
        if all_branches {
            revwalk.push_glob("refs/heads/*").ok();
            revwalk.push_glob("refs/remotes/*").ok();
            revwalk.push_glob("refs/tags/*").ok();
        } else {
            revwalk.push_head().ok();
        }
        let mut commits = Vec::new();
        for oid in revwalk.take(limit) {
            let oid = oid.map_err(|e| format!("Failed to walk revision: {}", e))?;
            let commit = repo.find_commit(oid).map_err(|e| format!("Failed to find commit: {}", e))?;
            let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();
            let time = DateTime::from_timestamp(commit.time().seconds(), 0).map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default();
            commits.push((commit.id().to_string(), commit.summary().unwrap_or("").to_string(), commit.author().name().unwrap_or("").to_string(), time, parents));
        }
        Ok(commits)
    }

    pub fn get_status(path: &str) -> Result<Vec<(String, String)>, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut options = StatusOptions::new();
        options.include_untracked(true);
        let statuses = repo.statuses(Some(&mut options)).map_err(|e| e.message().to_string())?;
        let mut changed_files = Vec::new();
        for entry in statuses.iter() {
            let s = entry.status();
            let status_code = if s.is_index_new() || s.is_index_modified() || s.is_index_deleted() || s.is_index_renamed() || s.is_index_typechange() {
                "staged"
            } else {
                "unstaged"
            };
            if let Some(path) = entry.path() { changed_files.push((path.to_string(), status_code.to_string())); }
        }
        Ok(changed_files)
    }

    pub fn stash_save(path: &str) -> Result<(), String> {
        let mut repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        repo.stash_save(&sig, "GitAmicus Stash", Some(StashFlags::INCLUDE_UNTRACKED)).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn stash_pop(path: &str) -> Result<(), String> {
        let mut repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        repo.stash_pop(0, None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn undo_last_commit(path: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let target = repo.revparse_single("HEAD^").map_err(|e| e.message().to_string())?;
        repo.reset(&target, ResetType::Soft, None).map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn stage_files(path: &str, files: Vec<String>) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        for file in files { index.add_path(std::path::Path::new(&file)).map_err(|e| e.message().to_string())?; }
        index.write().map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn unstage_files(path: &str, files: Vec<String>) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let head = repo.head().map_err(|e| e.message().to_string())?;
        if let Some(head_oid) = head.target() {
            let target = repo.find_commit(head_oid).map_err(|e| e.message().to_string())?;
            let paths: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
            repo.reset_default(Some(target.as_object()), paths).map_err(|e| e.message().to_string())?;
        }
        Ok(())
    }

    pub fn create_commit(path: &str, message: &str) -> Result<(), String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut index = repo.index().map_err(|e| e.message().to_string())?;
        let tree_id = index.write_tree().map_err(|e| e.message().to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.message().to_string())?;
        let sig = repo.signature().map_err(|e| e.message().to_string())?;
        
        let parent_commit = match repo.head() {
            Ok(head) => head.peel_to_commit().ok(),
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch || e.code() == git2::ErrorCode::NotFound => {
                None
            }
            Err(e) => return Err(e.message().to_string()),
        };

        let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .map_err(|e| e.message().to_string())?;
        Ok(())
    }

    pub fn get_file_diff(path: &str, file_path: &str) -> Result<String, String> {
        let repo = Repository::open(path).map_err(|e| e.message().to_string())?;
        let mut opts = DiffOptions::new();
        opts.pathspec(file_path);
        let diff = repo.diff_index_to_workdir(None, Some(&mut opts)).map_err(|e| e.message().to_string())?;
        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() { '+' => "+", '-' => "-", ' ' => " ", _ => "" };
            diff_text.push_str(prefix);
            diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        }).map_err(|e| e.message().to_string())?;
        Ok(diff_text)
    }
}