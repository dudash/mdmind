use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExampleAsset {
    pub name: &'static str,
    pub file_name: &'static str,
    pub description: &'static str,
    pub contents: &'static str,
}

const README: &str = include_str!("../examples/README.md");

const ASSETS: &[ExampleAsset] = &[
    ExampleAsset {
        name: "demo",
        file_name: "demo.md",
        description: "Small starter map that shows the core format and flows.",
        contents: include_str!("../examples/demo.md"),
    },
    ExampleAsset {
        name: "product-status",
        file_name: "product-status.md",
        description: "Product planning and status tracking with ids, tags, and views.",
        contents: include_str!("../examples/product-status.md"),
    },
    ExampleAsset {
        name: "lantern-studio-map",
        file_name: "lantern-studio-map.md",
        description: "A larger operating map for a fictional live experience team.",
        contents: include_str!("../examples/lantern-studio-map.md"),
    },
    ExampleAsset {
        name: "game-world-moonwake",
        file_name: "game-world-moonwake.md",
        description: "Worldbuilding, quests, regions, and production for a narrative game.",
        contents: include_str!("../examples/game-world-moonwake.md"),
    },
    ExampleAsset {
        name: "novel-research-writing-map",
        file_name: "novel-research-writing-map.md",
        description: "Writing and research workflow with characters, themes, chapters, and notes.",
        contents: include_str!("../examples/novel-research-writing-map.md"),
    },
    ExampleAsset {
        name: "team-project-board",
        file_name: "team-project-board.md",
        description: "Cross-owner project tracking and execution planning.",
        contents: include_str!("../examples/team-project-board.md"),
    },
    ExampleAsset {
        name: "prompt-ops",
        file_name: "prompt-ops.md",
        description: "Prompt library and operational workflows for prompt-driven systems.",
        contents: include_str!("../examples/prompt-ops.md"),
    },
    ExampleAsset {
        name: "decision-log",
        file_name: "decision-log.md",
        description: "Decision tracking with options, tradeoffs, and rationale.",
        contents: include_str!("../examples/decision-log.md"),
    },
];

pub fn all() -> &'static [ExampleAsset] {
    ASSETS
}

pub fn readme_contents() -> &'static str {
    README
}

pub fn find(name_or_file: &str) -> Option<&'static ExampleAsset> {
    ASSETS.iter().find(|asset| {
        asset.name.eq_ignore_ascii_case(name_or_file)
            || asset.file_name.eq_ignore_ascii_case(name_or_file)
    })
}

pub fn discover_examples_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            candidates.push(bin_dir.join("examples"));
            if let Some(prefix) = bin_dir.parent() {
                candidates.push(prefix.join("share").join("mdmind").join("examples"));
            }
        }
    }

    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples"));

    candidates
        .into_iter()
        .find(|path| path.is_dir() && path.join("README.md").is_file())
}

#[cfg(test)]
mod tests {
    use super::{all, discover_examples_dir, find, readme_contents};

    #[test]
    fn every_bundled_example_has_map_contents() {
        for asset in all() {
            assert!(asset.file_name.ends_with(".md"));
            assert!(asset.contents.contains("- "));
        }
    }

    #[test]
    fn bundled_example_lookup_supports_name_and_file() {
        assert_eq!(
            find("demo").expect("demo example should exist").file_name,
            "demo.md"
        );
        assert_eq!(
            find("demo.md").expect("demo.md example should exist").name,
            "demo"
        );
    }

    #[test]
    fn examples_directory_is_discoverable_in_repo() {
        let path = discover_examples_dir().expect("repo examples should be discoverable");
        assert!(path.join("README.md").is_file());
        assert!(readme_contents().contains("Example Maps"));
    }
}
