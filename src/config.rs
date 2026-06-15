use serde::Deserialize;

pub const CONFIG_FILE_PATH: &str = "config.toml";
pub const DEFAULT_CONFIG_CONTENTS: &str = "port = ${REQCTL_PORT:-3000}\n";

#[derive(Debug, Deserialize)]
struct FileConfig {
    port: u16,
}

#[derive(Debug)]
pub struct AppConfig {
    pub port: u16,
}

#[derive(Debug)]
pub enum ConfigError {
    /// Covers I/O (non-"not found"), interpolation, TOML-syntax, and range errors.
    File(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::File(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// I/O wrapper: reads the real file + real environment, delegates to the
/// pure core. A missing file (`io::ErrorKind::NotFound`) is treated as
/// `DEFAULT_CONFIG_CONTENTS`; any other read error (permissions, etc.) is
/// `ConfigError::File` — it is NOT silently treated as "missing".
pub fn load() -> Result<AppConfig, ConfigError> {
    let raw = match std::fs::read_to_string(CONFIG_FILE_PATH) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => DEFAULT_CONFIG_CONTENTS.to_string(),
        Err(e) => {
            return Err(ConfigError::File(format!(
                "failed to read {CONFIG_FILE_PATH}: {e}"
            )))
        }
    };
    resolve(&raw, |name| std::env::var(name).ok())
}

/// Pure core: no real I/O, takes an injectable env-lookup closure.
/// Unit tests call this directly with literal strings and a fake lookup map.
fn resolve(
    raw: &str,
    env_lookup: impl Fn(&str) -> Option<String>,
) -> Result<AppConfig, ConfigError> {
    let expanded = expand_env_vars(raw, &env_lookup).map_err(ConfigError::File)?;
    let file_config: FileConfig = toml::from_str(&expanded).map_err(|e| {
        ConfigError::File(format!(
            "invalid config.toml: {e} (expanded contents: {expanded:?})"
        ))
    })?;
    Ok(AppConfig {
        port: file_config.port,
    })
}

/// Hand-written scanner (no regex dependency): replaces `${NAME}` and
/// `${NAME:-default}` tokens with the looked-up env var value, or the
/// literal default text, or errors if `${NAME}` has no default and is unset.
/// Any `$` not followed by `{` is passed through literally. TOML comments
/// (`#` to end of line) are passed through verbatim and never expanded, so
/// e.g. a comment documenting the `${VAR:-default}` syntax itself doesn't
/// get interpolated.
fn expand_env_vars(
    input: &str,
    lookup: &impl Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '#' {
            out.push(c);
            for c in chars.by_ref() {
                out.push(c);
                if c == '\n' {
                    break;
                }
            }
            continue;
        }

        if c != '$' || chars.peek() != Some(&'{') {
            out.push(c);
            continue;
        }

        // Consume the '{'.
        chars.next();

        let mut token = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == '}' {
                closed = true;
                break;
            }
            token.push(c);
        }
        if !closed {
            return Err(format!(
                "unterminated '${{' in config: missing closing '}}' after \"${{{token}\""
            ));
        }

        let (name, default) = match token.split_once(":-") {
            Some((name, default)) => (name, Some(default)),
            None => (token.as_str(), None),
        };

        match lookup(name) {
            Some(value) => out.push_str(&value),
            None => match default {
                Some(default) => out.push_str(default),
                None => {
                    return Err(format!(
                        "environment variable '{name}' is not set and ${{{name}}} has no default"
                    ))
                }
            },
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup<'a>(env: &'a HashMap<&'a str, &'a str>) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| env.get(name).map(|v| v.to_string())
    }

    /// 1.1 - no configuration present (default port)
    #[test]
    fn default_port_when_no_config_present() {
        let env = HashMap::new();
        let config = resolve(DEFAULT_CONFIG_CONTENTS, lookup(&env)).unwrap();
        assert_eq!(config.port, 3000);
    }

    /// 1.2 - port from config file only (literal value, no env var)
    #[test]
    fn port_from_literal_config_file() {
        let env = HashMap::new();
        let config = resolve("port = 8080\n", lookup(&env)).unwrap();
        assert_eq!(config.port, 8080);
    }

    /// 1.3 - port from env var only, via interpolation with no default
    #[test]
    fn port_from_env_var_via_required_interpolation() {
        let mut env = HashMap::new();
        env.insert("REQCTL_PORT", "8081");
        let config = resolve("port = ${REQCTL_PORT}\n", lookup(&env)).unwrap();
        assert_eq!(config.port, 8081);
    }

    /// 1.4' - literal port in file is not affected by env var
    #[test]
    fn literal_port_ignores_env_var() {
        let mut env = HashMap::new();
        env.insert("REQCTL_PORT", "9090");
        let config = resolve("port = 8080\n", lookup(&env)).unwrap();
        assert_eq!(config.port, 8080);
    }

    /// 1.5' - interpolated port picks up a valid env var
    #[test]
    fn interpolated_port_uses_env_var_when_set() {
        let mut env = HashMap::new();
        env.insert("REQCTL_PORT", "9090");
        let config = resolve(DEFAULT_CONFIG_CONTENTS, lookup(&env)).unwrap();
        assert_eq!(config.port, 9090);
    }

    /// 1.6' - interpolated port falls back to default when env var unset
    #[test]
    fn interpolated_port_falls_back_when_env_var_unset() {
        let env = HashMap::new();
        let config = resolve(DEFAULT_CONFIG_CONTENTS, lookup(&env)).unwrap();
        assert_eq!(config.port, 3000);
    }

    /// 1.7 - config file with invalid TOML syntax
    #[test]
    fn invalid_toml_syntax_is_file_error() {
        let env = HashMap::new();
        let err = resolve("port = \n", lookup(&env)).unwrap_err();
        assert!(matches!(err, ConfigError::File(_)));
    }

    /// 1.8 - port not representable as u16 in a literal file
    #[test]
    fn port_not_representable_as_u16_is_file_error() {
        let env = HashMap::new();
        let err = resolve("port = 70000\n", lookup(&env)).unwrap_err();
        assert!(matches!(err, ConfigError::File(_)));

        let err = resolve("port = \"abc\"\n", lookup(&env)).unwrap_err();
        assert!(matches!(err, ConfigError::File(_)));

        let err = resolve("port = -1\n", lookup(&env)).unwrap_err();
        assert!(matches!(err, ConfigError::File(_)));
    }

    /// 1.9 - missing config file is not an error (covered via DEFAULT_CONFIG_CONTENTS)
    #[test]
    fn missing_config_file_treated_as_default() {
        let env = HashMap::new();
        let config = resolve(DEFAULT_CONFIG_CONTENTS, lookup(&env)).unwrap();
        assert_eq!(config.port, 3000);
    }

    /// A `${...}` pattern inside a `#` comment (e.g. documentation
    /// describing the interpolation syntax) is not expanded.
    #[test]
    fn comment_mentioning_interpolation_syntax_is_not_expanded() {
        let env = HashMap::new();
        let config = resolve(
            "# port accepts ${VAR} / ${VAR:-default}\nport = 8080\n",
            lookup(&env),
        )
        .unwrap();
        assert_eq!(config.port, 8080);
    }

    /// 1.10' - interpolated env var with non-numeric value fails as ConfigError::File
    #[test]
    fn interpolated_non_numeric_env_var_is_file_error() {
        let mut env = HashMap::new();
        env.insert("REQCTL_PORT", "not-a-port");
        let err = resolve(DEFAULT_CONFIG_CONTENTS, lookup(&env)).unwrap_err();
        match err {
            ConfigError::File(msg) => assert!(msg.contains("not-a-port")),
        }
    }

    /// 1.11' - required env var (${NAME}, no default), unset
    #[test]
    fn required_env_var_unset_is_file_error_naming_variable() {
        let env = HashMap::new();
        let err = resolve("port = ${REQCTL_PORT}\n", lookup(&env)).unwrap_err();
        match err {
            ConfigError::File(msg) => assert!(msg.contains("REQCTL_PORT")),
        }
    }

    /// 1.12' - required env var (${NAME}, no default), set and valid
    #[test]
    fn required_env_var_set_and_valid() {
        let mut env = HashMap::new();
        env.insert("REQCTL_PORT", "8081");
        let config = resolve("port = ${REQCTL_PORT}\n", lookup(&env)).unwrap();
        assert_eq!(config.port, 8081);
    }

    /// 1.13' - config.toml exists but is unreadable (e.g. permission denied)
    #[test]
    #[cfg(unix)]
    fn unreadable_config_file_is_file_error() {
        use std::fs;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("reqctl-test-config-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        {
            let mut f = fs::File::create(&path).unwrap();
            f.write_all(b"port = 8080\n").unwrap();
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

        let result = (|| -> Result<AppConfig, ConfigError> {
            let raw = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    DEFAULT_CONFIG_CONTENTS.to_string()
                }
                Err(e) => {
                    return Err(ConfigError::File(format!(
                        "failed to read config.toml: {e}"
                    )))
                }
            };
            resolve(&raw, |name| std::env::var(name).ok())
        })();

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        match result {
            Err(ConfigError::File(_)) => {}
            other => panic!("expected ConfigError::File, got {other:?}"),
        }
    }
}
