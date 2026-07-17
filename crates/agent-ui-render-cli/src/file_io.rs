use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

use anyhow::Context;

const HANDOFF_MARKER: &str = "agent-ui-render managed handoff\nformat=1\n";
const HANDOFF_MARKER_NAME: &str = ".agent-ui-render-managed";
const HANDOFF_LOCK_NAME: &str = ".agent-ui-renderer.lock";

pub fn atomic_write_text(path: &Path, content: &str) -> anyhow::Result<()> {
    ensure_parent_dir(path)?;
    let parent = parent_dir(path);
    let staging = tempfile::Builder::new()
        .prefix(".agent-ui-render-write-")
        .tempdir_in(parent)
        .with_context(|| format!("failed to create staging directory in {}", parent.display()))?;
    let staged_path = staging.path().join("output");
    let mut staged = File::create(&staged_path)
        .with_context(|| format!("failed to create staged output for {}", path.display()))?;
    staged
        .write_all(content.as_bytes())
        .with_context(|| format!("failed to stage {}", path.display()))?;
    staged
        .sync_all()
        .with_context(|| format!("failed to sync staged output for {}", path.display()))?;
    drop(staged);

    replace_file(&staged_path, path, staging.path())?;
    sync_parent(parent);
    Ok(())
}

pub fn replace_vue_handoff(
    output_path: &Path,
    wrapper: &str,
    files: &[(&str, &str)],
    force: bool,
) -> anyhow::Result<PathBuf> {
    ensure_parent_dir(output_path)?;
    let output_dir = parent_dir(output_path);
    let renderer_dir = output_dir.join("agent-ui-renderer");
    if output_path == renderer_dir {
        anyhow::bail!(
            "Vue wrapper output path must not be {}",
            renderer_dir.display()
        );
    }
    let _handoff_lock = acquire_handoff_lock(output_dir)?;
    ensure_replaceable_renderer(&renderer_dir, files, force)?;

    let transaction = tempfile::Builder::new()
        .prefix(".agent-ui-render-handoff-")
        .tempdir_in(output_dir)
        .with_context(|| {
            format!(
                "failed to create handoff staging directory in {}",
                output_dir.display()
            )
        })?;
    let staged_renderer = transaction.path().join("agent-ui-renderer");
    fs::create_dir(&staged_renderer).with_context(|| {
        format!(
            "failed to create staged renderer {}",
            staged_renderer.display()
        )
    })?;
    for (relative, content) in files {
        let relative = validated_relative_path(relative)?;
        let path = staged_renderer.join(relative);
        write_staged_text(&path, content)?;
    }
    write_staged_text(&staged_renderer.join(HANDOFF_MARKER_NAME), HANDOFF_MARKER)?;

    let backup = transaction.path().join("previous-renderer");
    let had_existing = renderer_dir.exists();
    if had_existing {
        fs::rename(&renderer_dir, &backup).with_context(|| {
            format!(
                "failed to stage existing renderer {} for replacement",
                renderer_dir.display()
            )
        })?;
    }
    if let Err(error) = fs::rename(&staged_renderer, &renderer_dir) {
        if had_existing {
            let _ = fs::rename(&backup, &renderer_dir);
        }
        return Err(error)
            .with_context(|| format!("failed to install renderer {}", renderer_dir.display()));
    }

    if let Err(error) = atomic_write_text(output_path, wrapper) {
        let _ = fs::remove_dir_all(&renderer_dir);
        if had_existing {
            let _ = fs::rename(&backup, &renderer_dir);
        }
        return Err(error).context("failed to install Vue wrapper; restored previous handoff");
    }

    sync_parent(output_dir);
    Ok(renderer_dir)
}

fn ensure_replaceable_renderer(
    renderer_dir: &Path,
    files: &[(&str, &str)],
    force: bool,
) -> anyhow::Result<()> {
    if !renderer_dir.exists() || force {
        return Ok(());
    }
    if !renderer_dir.is_dir() {
        anyhow::bail!(
            "refusing to replace unmanaged path {}; pass --force to replace it",
            renderer_dir.display()
        );
    }
    let marker = renderer_dir.join(HANDOFF_MARKER_NAME);
    if marker.is_file() && fs::read_to_string(&marker).ok().as_deref() == Some(HANDOFF_MARKER) {
        return Ok(());
    }

    let expected = files
        .iter()
        .map(|(relative, _)| validated_relative_path(relative).map(Path::to_path_buf))
        .collect::<anyhow::Result<BTreeSet<_>>>()?;
    let mut actual = BTreeSet::new();
    collect_relative_files(renderer_dir, renderer_dir, &mut actual)?;
    if actual == expected {
        return Ok(());
    }

    anyhow::bail!(
        "refusing to replace unmanaged directory {}; move it or pass --force",
        renderer_dir.display()
    )
}

fn collect_relative_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to inspect {}", directory.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_symlink() {
            anyhow::bail!(
                "refusing to replace handoff containing symlink {}",
                path.display()
            );
        }
        if file_type.is_dir() {
            collect_relative_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.insert(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn validated_relative_path(path: &str) -> anyhow::Result<&Path> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        anyhow::bail!("invalid embedded handoff path {}", path.display());
    }
    Ok(path)
}

fn write_staged_text(path: &Path, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create staged directory {}", parent.display()))?;
    }
    let mut file = File::create(path)
        .with_context(|| format!("failed to create staged file {}", path.display()))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("failed to write staged file {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync staged file {}", path.display()))
}

#[cfg(not(windows))]
fn replace_file(staged: &Path, destination: &Path, _transaction: &Path) -> anyhow::Result<()> {
    fs::rename(staged, destination).with_context(|| {
        format!(
            "failed to atomically replace {} with staged output",
            destination.display()
        )
    })
}

#[cfg(windows)]
fn replace_file(staged: &Path, destination: &Path, transaction: &Path) -> anyhow::Result<()> {
    let backup = transaction.join("previous-output");
    let had_existing = destination.exists();
    if had_existing {
        fs::rename(destination, &backup).with_context(|| {
            format!("failed to stage existing output {}", destination.display())
        })?;
    }
    if let Err(error) = fs::rename(staged, destination) {
        if had_existing {
            let _ = fs::rename(&backup, destination);
        }
        return Err(error).with_context(|| {
            format!(
                "failed to replace output {}; previous output restored",
                destination.display()
            )
        });
    }
    Ok(())
}

fn acquire_handoff_lock(output_dir: &Path) -> anyhow::Result<File> {
    let path = output_dir.join(HANDOFF_LOCK_NAME);
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("failed to open handoff lock {}", path.display()))?;
    file.lock()
        .with_context(|| format!("failed to acquire handoff lock {}", path.display()))?;
    Ok(file)
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    let parent = parent_dir(path);
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn sync_parent(parent: &Path) {
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_complete_file() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let output = temp.path().join("report.html");
        fs::write(&output, "old")?;

        atomic_write_text(&output, "complete new output")?;

        assert_eq!(fs::read_to_string(output)?, "complete new output");
        Ok(())
    }

    #[test]
    fn concurrent_handoffs_keep_wrapper_and_renderer_consistent() -> anyhow::Result<()> {
        use std::{sync::Barrier, thread};

        let temp = tempfile::tempdir()?;
        let output = temp.path().join("Report.vue");
        let renderer_path = temp.path().join("agent-ui-renderer/AgentUiRenderer.vue");

        for _ in 0..16 {
            let barrier = Barrier::new(3);
            let results = thread::scope(|scope| {
                let handles = [("wrapper-a", "renderer-a"), ("wrapper-b", "renderer-b")]
                    .into_iter()
                    .map(|(wrapper, renderer)| {
                        let barrier = &barrier;
                        let output = &output;
                        scope.spawn(move || {
                            barrier.wait();
                            replace_vue_handoff(
                                output,
                                wrapper,
                                &[("AgentUiRenderer.vue", renderer)],
                                false,
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                barrier.wait();
                handles
                    .into_iter()
                    .map(|handle| match handle.join() {
                        Ok(result) => result,
                        Err(_) => Err(anyhow::anyhow!("handoff thread panicked")),
                    })
                    .collect::<Vec<_>>()
            });
            for result in results {
                result?;
            }

            let wrapper = fs::read_to_string(&output)?;
            let renderer = fs::read_to_string(&renderer_path)?;
            assert!(
                (wrapper == "wrapper-a" && renderer == "renderer-a")
                    || (wrapper == "wrapper-b" && renderer == "renderer-b"),
                "wrapper {wrapper:?} and renderer {renderer:?} came from different transactions"
            );
        }
        Ok(())
    }

    #[test]
    fn handoff_rollback_restores_previous_renderer() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let output = temp.path().join("Report.vue");
        fs::create_dir(&output)?;
        let renderer = temp.path().join("agent-ui-renderer");
        fs::create_dir(&renderer)?;
        fs::write(renderer.join(HANDOFF_MARKER_NAME), HANDOFF_MARKER)?;
        fs::write(renderer.join("AgentUiRenderer.vue"), "old renderer")?;

        let result = replace_vue_handoff(
            &output,
            "new wrapper",
            &[("AgentUiRenderer.vue", "new renderer")],
            false,
        );

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(renderer.join("AgentUiRenderer.vue"))?,
            "old renderer"
        );
        Ok(())
    }
}
