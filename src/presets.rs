use crate::fs_utils::expand_env_vars;
use crate::models::PresetGame;
use std::path::{Path, PathBuf};

pub struct PresetDefinition {
    pub name: &'static str,
    pub steam_app_id: Option<&'static str>,
    pub steam_name_keyword: Option<&'static str>,
    pub path_candidates: &'static [&'static str],
    pub install_subpath: Option<&'static [&'static str]>,
    pub note: Option<&'static str>,
}

pub const PRESET_DEFINITIONS: &[PresetDefinition] = &[
    PresetDefinition {
        name: "赛博朋克2077",
        steam_app_id: Some("1091500"),
        steam_name_keyword: Some("cyberpunk 2077"),
        path_candidates: &["%USERPROFILE%/Saved Games/CD Projekt Red/Cyberpunk 2077"],
        install_subpath: None,
        note: None,
    },
    PresetDefinition {
        name: "艾尔登法环",
        steam_app_id: Some("1245620"),
        steam_name_keyword: Some("elden ring"),
        path_candidates: &["%APPDATA%/EldenRing"],
        install_subpath: None,
        note: None,
    },
    PresetDefinition {
        name: "黑神话悟空",
        steam_app_id: Some("2358720"),
        steam_name_keyword: Some("black myth"),
        path_candidates: &[
            "%PROGRAMFILES(X86)%/Steam/steamapps/common/BlackMythWukong/b1/Saved/SaveGames",
        ],
        install_subpath: Some(&["b1", "Saved", "SaveGames"]),
        note: Some("该路径与安装位置和平台有关，只作为候选路径。"),
    },
    PresetDefinition {
        name: "巫师3",
        steam_app_id: Some("292030"),
        steam_name_keyword: Some("witcher 3"),
        path_candidates: &["%USERPROFILE%/Documents/The Witcher 3/gamesaves"],
        install_subpath: None,
        note: None,
    },
    PresetDefinition {
        name: "星露谷物语",
        steam_app_id: Some("413150"),
        steam_name_keyword: Some("stardew valley"),
        path_candidates: &["%APPDATA%/StardewValley/Saves"],
        install_subpath: None,
        note: None,
    },
    PresetDefinition {
        name: "只狼",
        steam_app_id: Some("814380"),
        steam_name_keyword: Some("sekiro"),
        path_candidates: &["%APPDATA%/Sekiro"],
        install_subpath: None,
        note: None,
    },
    PresetDefinition {
        name: "博德之门3",
        steam_app_id: Some("1086940"),
        steam_name_keyword: Some("baldur's gate 3"),
        path_candidates: &["%LOCALAPPDATA%/Larian Studios/Baldur's Gate 3/PlayerProfiles"],
        install_subpath: None,
        note: None,
    },
    PresetDefinition {
        name: "幻兽帕鲁",
        steam_app_id: Some("1623730"),
        steam_name_keyword: Some("palworld"),
        path_candidates: &["%LOCALAPPDATA%/Pal/Saved/SaveGames"],
        install_subpath: None,
        note: None,
    },
];

pub fn built_in_presets() -> Vec<PresetGame> {
    PRESET_DEFINITIONS
        .iter()
        .map(|def| PresetGame {
            name: def.name.to_owned(),
            path_candidates: def.path_candidates.iter().map(|&s| s.to_owned()).collect(),
            note: def.note.map(|s| s.to_owned()),
        })
        .collect()
}

pub fn first_expanded_path(preset: &PresetGame) -> Option<PathBuf> {
    preset
        .path_candidates
        .first()
        .map(|path| PathBuf::from(expand_env_vars(path)))
}

pub fn find_steam_known_paths(
    app_id: &str,
    name: &str,
    install_dir: &Path,
) -> Vec<PathBuf> {
    let lower_name = name.to_lowercase();
    let mut paths = Vec::new();

    for def in PRESET_DEFINITIONS {
        let app_id_matches = def.steam_app_id.is_some_and(|id| id == app_id);
        let name_matches = def.steam_name_keyword.is_some_and(|kw| lower_name.contains(kw));

        if app_id_matches || name_matches {
            if let Some(subparts) = def.install_subpath {
                let mut path = install_dir.to_path_buf();
                for part in subparts {
                    path = path.join(part);
                }
                paths.push(path);
            } else {
                for candidate in def.path_candidates {
                    paths.push(PathBuf::from(expand_env_vars(candidate)));
                }
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_and_steam_matching_are_consistent() {
        let presets = built_in_presets();
        assert_eq!(presets.len(), 8);

        let paths = find_steam_known_paths("1245620", "ELDEN RING", Path::new("test"));
        assert_eq!(paths.len(), 1);

        let paths_wukong = find_steam_known_paths("2358720", "Black Myth: Wukong", Path::new("C:/games/b1"));
        assert_eq!(paths_wukong.len(), 1);
        assert!(paths_wukong[0].ends_with("SaveGames"));
    }
}
