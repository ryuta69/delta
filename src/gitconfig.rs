use std::collections::HashMap;
use std::process;

use crate::cli;
use crate::preset;

pub fn get_option_value__string(
    option_name: &str,
    builtin_presets: HashMap<String, preset::BuiltinPreset>,
    opt: &cli::Opt,
    git_config: &mut Option<git2::Config>,
) -> Option<String> {
    match opt.presets.as_deref().map(str::to_lowercase) {
        Some(presets) => {
            for preset in presets.split_whitespace().rev() {
                if let Some(value) = get_option_value_for_preset__string(
                    option_name,
                    &preset,
                    &builtin_presets,
                    opt,
                    git_config,
                ) {
                    return Some(value);
                }
            }
            None
        }
        None => None,
    }
}

// If the value for option name n was not supplied on the command line, then a search is performed
// of the following locations in the order specified, and the first value encountered is used:
// 1. The value of n under p interpreted as a user-supplied preset (i.e. git config value delta.$p.$n)
// 2. The value for n under p interpreted as a builtin preset
// 3. The value for n in the main git config section for delta (i.e. git config value delta.$n)
fn get_option_value_for_preset__string(
    option_name: &str,
    preset: &str,
    builtin_presets: &HashMap<String, preset::BuiltinPreset>,
    opt: &cli::Opt,
    git_config: &mut Option<git2::Config>,
) -> Option<String> {
    if let Some(git_config) = git_config {
        let git_config = git_config.snapshot().unwrap_or_else(|err| {
            eprintln!("Failed to read git config: {}", err);
            process::exit(1)
        });
        if let Some(value) =
            git_config_get_2::<String>(&format!("delta.{}.{}", preset, option_name), git_config)
        {
            return Some(value);
        }
    }
    if let Some(builtin_preset) = builtin_presets.get(preset) {
        if let Some(value_function) = builtin_preset.get(option_name) {
            return Some(value_function(opt, &git_config));
        }
    }
    if let Some(git_config) = git_config {
        let git_config = git_config.snapshot().unwrap_or_else(|err| {
            eprintln!("Failed to read git config: {}", err);
            process::exit(1)
        });
        if let Some(value) =
            git_config_get_2::<String>(&format!("delta.{}", option_name), git_config)
        {
            return Some(value);
        }
    }
    return None;
}

trait GitConfigGet {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self>
    where
        Self: Sized;
}

impl GitConfigGet for String {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_string(key).ok()
    }
}

impl GitConfigGet for bool {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_bool(key).ok()
    }
}

impl GitConfigGet for i64 {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_i64(key).ok()
    }
}

fn git_config_get_2<T>(key: &str, git_config: git2::Config) -> Option<T>
where
    T: GitConfigGet,
{
    T::git_config_get(key, &git_config)
}

#[macro_use]
mod set_options {
    // set_options<T> implementations

    macro_rules! set_options__string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                let builtin_presets = $crate::preset::make_builtin_presets();
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = $crate::gitconfig::get_option_value__string($option_name, builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value;
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__option_string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                let keys = $crate::gitconfig::make_git_config_keys($option_name, $opt.presets.as_deref());
                if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    $opt.$field_ident = $crate::gitconfig::git_config_get::_string(keys, $git_config)
                                        .or_else(|| $crate::gitconfig::get_default::_string($option_name, &$opt)
                                                    .or_else(|| $opt.$field_ident.as_deref().map(str::to_string)));
                };
            )*
	    };
    }

    macro_rules! set_options__bool {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                let keys = $crate::gitconfig::make_git_config_keys($option_name, $opt.presets.as_deref());
                if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    $opt.$field_ident =
                        $crate::gitconfig::git_config_get::_bool(keys, $git_config)
                        .unwrap_or_else(|| $crate::gitconfig::get_default::_bool($option_name, &$opt)
                                           .unwrap_or($opt.$field_ident));
                };
            )*
	    };
    }

    macro_rules! set_options__f64 {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                let keys = $crate::gitconfig::make_git_config_keys($option_name, $opt.presets.as_deref());
                if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    let get_default = || $crate::gitconfig::get_default::_f64($option_name, &$opt)
                                         .unwrap_or($opt.$field_ident);
                    $opt.$field_ident = match $crate::gitconfig::git_config_get::_string(keys, $git_config) {
                        Some(s) => s.parse::<f64>().unwrap_or_else(|_| get_default()),
                        None => get_default(),
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__usize {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                let keys = $crate::gitconfig::make_git_config_keys($option_name, $opt.presets.as_deref());
                if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    $opt.$field_ident = match $crate::gitconfig::git_config_get::_i64(keys, $git_config) {
                        Some(int) => int as usize,
                        None => $crate::gitconfig::get_default::_usize($option_name, &$opt)
                                .unwrap_or($opt.$field_ident),
                    }
                };
            )*
	    };
    }
}

pub mod git_config_get {
    use git2;

    macro_rules! _git_config_get {
        ($keys:expr, $git_config:expr, $getter:ident) => {
            match $git_config {
                Some(git_config) => {
                    let git_config = git_config.snapshot().unwrap();
                    for key in $keys {
                        let entry = git_config.$getter(&key);
                        if let Ok(entry) = entry {
                            return Some(entry);
                        }
                    }
                    None
                }
                None => None,
            }
        };
    }

    /// Get String value from gitconfig
    pub fn _string(keys: Vec<String>, git_config: &mut Option<git2::Config>) -> Option<String> {
        _git_config_get!(keys, git_config, get_string)
    }

    /// Get bool value from gitconfig
    pub fn _bool(keys: Vec<String>, git_config: &mut Option<git2::Config>) -> Option<bool> {
        _git_config_get!(keys, git_config, get_bool)
    }

    /// Get i64 value from gitconfig
    pub fn _i64(keys: Vec<String>, git_config: &mut Option<git2::Config>) -> Option<i64> {
        _git_config_get!(keys, git_config, get_i64)
    }
}

pub fn make_git_config_keys(key: &str, _presets: Option<&str>) -> Vec<String> {
    vec![format!("delta.{}", key)]
}

pub mod get_default {
    use crate::cli;

    pub fn _string(_option_name: &str, _opt: &cli::Opt) -> Option<String> {
        None
    }

    pub fn _bool(_option_name: &str, _opt: &cli::Opt) -> Option<bool> {
        None // bool preset defaults not needed yet
    }

    pub fn _f64(_option_name: &str, _opt: &cli::Opt) -> Option<f64> {
        None // f64 preset defaults not needed yet
    }

    pub fn _usize(_option_name: &str, _opt: &cli::Opt) -> Option<usize> {
        None // usize preset defaults not needed yet
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::Path;

    use git2;
    use itertools;

    use crate::config;
    use crate::style::{DecorationStyle, Style};

    #[test]
    fn test_main_section() {
        let git_config_contents = b"
[delta]
    minus-style = blue
";
        let git_config_path = "delta__test_main_section.gitconfig";

        // First check that it doesn't default to blue, because that's going to be used to signal
        // that gitconfig has set the style.
        assert_ne!(make_config(&[], None, None).minus_style, make_style("blue"));

        // Check that --minus-style is honored as we expect.
        assert_eq!(
            make_config(&["--minus-style", "red"], None, None).minus_style,
            make_style("red")
        );

        // Check that gitconfig does not override a command line argument
        assert_eq!(
            make_config(
                &["--minus-style", "red"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("red")
        );

        // Finally, check that gitconfig is honored when not overridden by a command line argument.
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_preset() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset\"]
    minus-style = green
";
        let git_config_path = "delta__test_preset.gitconfig";

        // Without --presets the main section takes effect
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        // With --presets the preset takes effect
        assert_eq!(
            make_config(
                &["--presets", "my-preset"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_multiple_presets() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset-1\"]
    minus-style = green

[delta \"my-preset-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_multiple_presets.gitconfig";

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1 my-preset-2"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-2 my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_invalid_presets() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset-1\"]
    minus-style = green

[delta \"my-preset-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_invalid_presets.gitconfig";

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("blue")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1 my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-x my-preset-2 my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_highlight_defaults() {
        let config = make_config(&["--presets", "diff-highlight"], None, None);

        assert_eq!(config.minus_style, make_style("red"));
        assert_eq!(config.minus_non_emph_style, make_style("red"));
        assert_eq!(config.minus_emph_style, make_emph_style("red reverse"));
        assert_eq!(config.zero_style, make_style(""));
        assert_eq!(config.plus_style, make_style("green"));
        assert_eq!(config.plus_non_emph_style, make_style("green"));
        assert_eq!(config.plus_emph_style, make_emph_style("green reverse"));
    }

    #[test]
    fn test_diff_highlight_respects_gitconfig() {
        let git_config_contents = b"
[color \"diff\"]
    old = red bold
    new = green bold

[color \"diff-highlight\"]
    oldNormal = ul red bold
    oldHighlight = red bold 52
    newNormal = ul green bold
    newHighlight = green bold 22
";
        let git_config_path = "delta__test_diff_highlight.gitconfig";

        let config = make_config(
            &["--presets", "diff-highlight"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(config.minus_style, make_style("red bold"));
        assert_eq!(config.minus_non_emph_style, make_style("ul red bold"));
        assert_eq!(config.minus_emph_style, make_emph_style("red bold 52"));
        assert_eq!(config.zero_style, make_style(""));
        assert_eq!(config.plus_style, make_style("green bold"));
        assert_eq!(config.plus_non_emph_style, make_style("ul green bold"));
        assert_eq!(config.plus_emph_style, make_emph_style("green bold 22"));

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_defaults() {
        let config = make_config(&["--presets", "diff-so-fancy"], None, None);

        assert_eq!(
            config.commit_style.ansi_term_style,
            make_style("bold yellow").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            make_decoration_style("none")
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );
        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("bold yellow ul ol")
        );

        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            make_style("bold syntax").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            make_decoration_style("magenta box")
        );
    }

    #[test]
    fn test_diff_so_fancy_respects_git_config() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse
";
        let git_config_path = "delta__test_diff_so_fancy.gitconfig";

        let config = make_config(
            &["--presets", "diff-so-fancy some-other-preset"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.commit_style.ansi_term_style,
            make_style("yellow bold").ansi_term_style
        );
        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            make_style("magenta bold").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            make_decoration_style("none")
        );
        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("yellow bold ul ol")
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            make_decoration_style("magenta box")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_obeys_preset_precedence_rules() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse

[delta \"decorations\"]
    commit-decoration-style = bold box ul
    file-style = bold 19 ul
    file-decoration-style = none
";
        let git_config_path = "delta__test_diff_so_fancy_obeys_preset_precedence_rules.gitconfig";

        let config = make_config(
            &["--presets", "decorations diff-so-fancy"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("yellow bold ul ol")
        );

        let config = make_config(
            &["--presets", "diff-so-fancy decorations"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("ul bold 19").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("none")
        );

        remove_file(git_config_path).unwrap();
    }

    fn make_style(s: &str) -> Style {
        _make_style(s, false)
    }

    fn make_emph_style(s: &str) -> Style {
        _make_style(s, true)
    }

    fn _make_style(s: &str, is_emph: bool) -> Style {
        Style::from_str(s, None, None, None, true, is_emph)
    }

    fn make_decoration_style(s: &str) -> DecorationStyle {
        DecorationStyle::from_str(s, true)
    }

    fn make_git_config(contents: &[u8], path: &str) -> git2::Config {
        let path = Path::new(path);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        git2::Config::open(&path).unwrap()
    }

    fn make_config<'a>(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        path: Option<&str>,
    ) -> config::Config<'a> {
        let args: Vec<&str> = itertools::chain(
            &["/dev/null", "/dev/null", "--24-bit-color", "always"],
            args,
        )
        .map(|s| *s)
        .collect();
        let mut git_config = match (git_config_contents, path) {
            (Some(contents), Some(path)) => Some(make_git_config(contents, path)),
            _ => None,
        };
        config::Config::from_args(&args, &mut git_config)
    }
}
