use anyhow::{Result, bail};
use ignore::WalkBuilder;
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use std::path::{Path, PathBuf};

pub fn walker(root: &Path, respect: bool, files: &[PathBuf]) -> Result<ignore::Walk> {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(respect)
        .hidden(false)
        .follow_links(false);
    for file in files {
        if let Some(error) = builder.add_ignore(file) {
            bail!("{}: {error}", file.display());
        }
    }
    Ok(builder.build())
}

pub struct Candidate {
    pub path: PathBuf,
    pub label: String,
    search: String,
    hidden: bool,
    depth: usize,
}

impl Candidate {
    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn new(path: PathBuf, root: &Path, directory: bool) -> Self {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let search = if relative.as_os_str().is_empty() {
            ".".into()
        } else {
            relative.to_string_lossy().into_owned()
        };
        let hidden = relative
            .components()
            .any(|part| part.as_os_str().to_string_lossy().starts_with('.'));
        let depth = relative.components().count();
        let mut label: String = search
            .chars()
            .flat_map(|c| {
                if c.is_control() {
                    c.escape_default().collect::<Vec<_>>()
                } else {
                    vec![c]
                }
            })
            .collect();
        if directory {
            label.push('/');
        }
        Self {
            path,
            label,
            search,
            hidden,
            depth,
        }
    }
}

fn explicit_hidden(query: &str) -> bool {
    query == "."
        || query
            .split_whitespace()
            .flat_map(|word| word.split('/'))
            .any(|part| part.starts_with('.') && part != "." && part != "..")
}

pub fn rank(candidates: &[Candidate], query: &str) -> Vec<usize> {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let mut buffer = Vec::new();
    let hidden_penalty = !explicit_hidden(query);
    let mut matches: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let score =
                pattern.score(Utf32Str::new(&candidate.search, &mut buffer), &mut matcher)?;
            let basename = candidate
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let exact = basename.eq_ignore_ascii_case(query)
                || basename
                    .strip_prefix('.')
                    .is_some_and(|name| name.eq_ignore_ascii_case(query));
            let score = i64::from(score) + if exact { 32 } else { 0 }
                - if candidate.hidden && hidden_penalty {
                    12
                } else {
                    0
                };
            Some((index, score))
        })
        .collect();
    matches.sort_unstable_by(|(a, sa), (b, sb)| {
        sb.cmp(sa)
            .then_with(|| candidates[*a].depth.cmp(&candidates[*b].depth))
            .then_with(|| {
                candidates[*a]
                    .search
                    .len()
                    .cmp(&candidates[*b].search.len())
            })
            .then_with(|| candidates[*a].path.cmp(&candidates[*b].path))
    });
    matches.into_iter().map(|(index, _)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_and_ignored_paths_remain_accessible() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::create_dir(root.join(".config")).unwrap();
        std::fs::write(root.join(".config/app"), "").unwrap();
        std::fs::write(root.join(".gitignore"), "build\n").unwrap();
        std::fs::write(root.join("build"), "").unwrap();
        let collect = |respect| {
            walker(root, respect, &[])
                .unwrap()
                .map(|entry| entry.unwrap().into_path())
                .collect::<Vec<_>>()
        };
        assert!(collect(false).contains(&root.join("build")));
        assert!(!collect(true).contains(&root.join("build")));
        assert!(collect(true).contains(&root.join(".config/app")));
        let ignore = root.join("rules");
        std::fs::write(&ignore, ".config/\n").unwrap();
        assert!(
            !walker(root, false, &[ignore])
                .unwrap()
                .any(|entry| entry.unwrap().path() == root.join(".config/app"))
        );
    }
    #[test]
    fn hidden_penalty_does_not_overrule_relevance() {
        let root = Path::new("/root");
        let paths = ["environment-notes.txt", ".env", "env", ".config/app"];
        let candidates: Vec<_> = paths
            .iter()
            .map(|p| Candidate::new(root.join(p), root, false))
            .collect();
        let ranked = rank(&candidates, "env");
        assert_eq!(ranked[0], 2);
        assert!(ranked.iter().position(|i| *i == 1) < ranked.iter().position(|i| *i == 0));
        assert_eq!(rank(&candidates, ".env")[0], 1);
        assert!(!explicit_hidden("./src/main.rs"));
        assert!(explicit_hidden("src/.cache"));
        assert!(!explicit_hidden("../src"));
    }
}
