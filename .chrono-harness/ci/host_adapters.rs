//! Deployment checks for the files selected by this host.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// A mise setup installs tools first and refreshes the rules only when selected.
pub fn setup_sequence(commands: &[String], instructions: bool) -> (accepted: bool)
    ensures accepted == (commands.len() == (if instructions { 3usize } else { 2usize })
        && commands@[0]@ == "sh .chrono-harness/install-tools.sh"@
        && (!instructions || commands@[1]@ == "mise exec -- chrono-instructions sync-rules --host-root ."@)
        && commands@[commands@.len() - 1]@ == "sh .chrono-harness/setup-check-retention.sh"@),
{
    if commands.len() != (if instructions { 3usize } else { 2usize }) { return false; }
    commands[0] == "sh .chrono-harness/install-tools.sh".to_string() && (!instructions
        || commands[1] == "mise exec -- chrono-instructions sync-rules --host-root .".to_string())
        && commands[commands.len() - 1] == "sh .chrono-harness/setup-check-retention.sh".to_string()
}

pub fn mise_manifest() -> (s: &'static str) {
    include_str!("../../.config/mise/conf.d/chrono-harness.toml")
}

/// The template renders these same rows into both mise platform tables.
pub fn installed_asset(linux: bool) -> (fields: (&'static str, usize))
    ensures linux ==> fields.0@ == "linux-x86_64"@ && fields.1 == 1,
        !linux ==> fields.0@ == "macos-aarch64"@ && fields.1 == 2,
{
    if linux {
        ("linux-x86_64", 1)
    } else {
        ("macos-aarch64", 2)
    }
}

/// mise reports file/key presence and applies the selected Rust-regex expression.
#[derive(Clone, Copy)]
pub enum InstalledValue { Pin, Use, Reject }

pub fn installed_value(present: bool, matches: bool) -> (action: InstalledValue)
    ensures !present ==> action == InstalledValue::Pin,
        present && matches ==> action == InstalledValue::Use,
        present && !matches ==> action == InstalledValue::Reject,
{
    if !present { InstalledValue::Pin } else if matches { InstalledValue::Use }
    else { InstalledValue::Reject }
}

pub fn installed_pattern(is_version: bool) -> (pattern: &'static str)
    ensures is_version ==> pattern@ == "^[0-9]+[.][0-9]+[.][0-9]+(-[A-Za-z0-9-]+([.][A-Za-z0-9-]+)*)?$"@,
        !is_version ==> pattern@ == "^sha256:[a-f0-9]{64}$"@,
{
    if is_version { "^[0-9]+[.][0-9]+[.][0-9]+(-[A-Za-z0-9-]+([.][A-Za-z0-9-]+)*)?$" } else { "^sha256:[a-f0-9]{64}$" }
}

pub fn installed_config() -> (s: &'static str) {
    include_str!("../../.config/mise/conf.d/selected-chrono-harness.toml")
}

pub fn deployed_tool_installer() -> (source: &'static str) {
    include_str!("../install-tools.sh")
}

pub fn installer_digest_matches(digest: &String) -> (accepted: bool)
    ensures accepted == (digest@ == "463052d115cab6d117f83b7546ee3c9481f42ab210c4347833668f84eb6d1941"@),
{
    *digest == "463052d115cab6d117f83b7546ee3c9481f42ab210c4347833668f84eb6d1941".to_string()
}

}

#[cfg(not(verus_keep_ghost))]
mod native {
    use super::*;

    #[test]
    fn deployed_tool_installer_matches_the_selected_template() {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(deployed_tool_installer().as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(
            installer_digest_matches(&digest),
            "install-tools.sh differs from the selected template"
        );
    }

    fn when_selected(condition: &str, installed: &str, pinned: &str) -> String {
        format!("{{% if {condition} %}}{installed}{{% else %}}{pinned}{{% endif %}}")
    }

    // Reconstruct the actions of the host's actual mise expressions for every
    // outcome of its built-in file, TOML lookup and regex operations.
    fn installed_field_expression(is_version: bool, command: &str, index: usize) -> String {
        let (value, lookup, field) = if is_version {
            ("version", format!(r#"data["{command}"][0]"#), "version")
        } else {
            (
                "hash",
                format!(r#"read_toml(path=vars.chrono_selection)["{command}"][{index}]"#),
                "checksum",
            )
        };
        let action = |present, matches| match installed_value(present, matches) {
            InstalledValue::Pin if is_version => String::new(),
            InstalledValue::Pin => r#"{{ PLATFORM["checksum"] }}"#.to_owned(),
            InstalledValue::Use => format!("{{{{ {value} }}}}"),
            InstalledValue::Reject => {
                format!(r#"{{{{ throw(message="Invalid installed release {field}") }}}}"#)
            }
        };
        let assign = format!("{{% set {value} = {lookup} %}}");
        let pattern = installed_pattern(is_version);
        let check = format!(r#"{{% if {value} is matching(pat="{pattern}") %}}"#);
        let yes = action(true, true);
        let no = action(true, false);
        let fallback = action(false, false);
        let end = "{% endif %}";
        let otherwise = "{% else %}";
        let inner = format!("{assign}{check}{yes}{otherwise}{no}{end}");
        if is_version {
            let file = "{% if vars.chrono_selection is exists %}{% set data = read_toml(path=vars.chrono_selection) %}{% endif %}";
            let has = format!(r#"{{% if data is defined and "{command}" in data %}}"#);
            format!("{file}{has}{inner}{end}{fallback}")
        } else {
            when_selected("SELECTED", &inner, &fallback)
        }
    }

    #[test]
    fn installed_assets_use_the_verified_platforms() {
        let document: toml::Value = toml::from_str(installed_config()).unwrap();
        assert_eq!(
            document["vars"]["chrono_selection"].as_str(),
            Some("{{ config_root }}/.chrono-harness/state/tool-selection/versions.toml")
        );
        assert_eq!(
            document["vars"]["chrono_pins"].as_str(),
            Some("{{ config_root }}/.config/mise/conf.d/chrono-harness.toml")
        );
        let entries = document["tools"].as_table().unwrap();
        assert_eq!(entries.len(), 21);
        for (key, tool) in entries {
            let command = key.strip_prefix("http:").unwrap();
            let repository = match command {
                "chrono-cache" => "chrono-harness/chrono-harness-cache",
                "chrono-ci" => "chrono-harness/chrono-harness-ci",
                "chrono-diagnostics" => "chrono-harness/chrono-harness-diagnostics",
                "chrono-distribution" => "chrono-harness/chrono-harness-distribution",
                "chrono-harness" => "chrono-harness/chrono-harness-runner",
                "chrono-host-bootstrap" => "chrono-harness/chrono-harness-host-bootstrap",
                "chrono-host-release" => "chrono-harness/chrono-harness-host-release",
                "chrono-inputs" => "chrono-harness/chrono-harness-inputs",
                "chrono-instructions" => "chrono-harness/chrono-harness-instructions",
                "chrono-judge-ci" => "chrono-harness/chrono-harness-judge-ci",
                "chrono-judge-cost" => "chrono-harness/chrono-harness-judge-cost",
                "chrono-judge-filemap" => "chrono-harness/chrono-harness-judge-filemap",
                "chrono-judge-mixed" => "chrono-harness/chrono-harness-judge-mixed",
                "chrono-judge-projects" => "chrono-harness/chrono-harness-judge-projects",
                "chrono-judge-registration" => "chrono-harness/chrono-harness-judge-registration",
                "chrono-judge-routes" => "chrono-harness/chrono-harness-judge-routes",
                "chrono-judge-workflow" => "chrono-harness/chrono-harness-judge-workflow",
                "chrono-pr" => "chrono-harness/chrono-harness-pr",
                "chrono-test-git" => "chrono-harness/chrono-harness-test-git",
                "chrono-test-git-profile" => "chrono-harness/chrono-harness-test-git",
                "chrono-worktree" => "chrono-harness/chrono-harness-worktree",
                _ => panic!("unexpected selected tool {command}"),
            };
            let variable = format!("selected_{}", command.replace('-', "_"));
            assert_eq!(
                document["vars"][&variable].as_str(),
                Some(installed_field_expression(true, command, 0).as_str())
            );
            let selected = format!("vars.{variable}");
            let value = format!("{{{{ {selected} }}}}");
            let pin_tool =
                format!(r#"read_toml(path=vars.chrono_pins)["tools"]["http:{command}"]"#);
            let pinned_version = format!(r#"{{{{ {pin_tool}["version"] }}}}"#);
            let version = when_selected(&selected, &value, &pinned_version);
            assert_eq!(tool["version"].as_str(), Some(version.as_str()));
            let platforms = tool["platforms"].as_table().unwrap();
            assert_eq!(platforms.len(), 2);
            for (name, linux) in [("linux-x64", true), ("macos-arm64", false)] {
                let (suffix, index) = installed_asset(linux);
                let pin = format!(r#"{pin_tool}["platforms"]["{name}"]"#);
                let asset = format!(
                    "https://github.com/{repository}/releases/download/v{value}/{command}-{suffix}"
                );
                let pinned_url = format!(r#"{{{{ {pin}["url"] }}}}"#);
                let url = when_selected(&selected, &asset, &pinned_url);
                let checksum = installed_field_expression(false, command, index)
                    .replace("SELECTED", &selected)
                    .replace("PLATFORM", &pin);
                assert_eq!(platforms[name]["url"].as_str(), Some(url.as_str()));
                assert_eq!(
                    platforms[name]["checksum"].as_str(),
                    Some(checksum.as_str())
                );
            }
        }
    }
    fn setup_matches(source: &str, instructions: bool) -> bool {
        let Ok(doc) = source.parse::<toml::Table>() else {
            return false;
        };
        let instruction_tool = doc
            .get("tools")
            .and_then(toml::Value::as_table)
            .is_some_and(|tools| tools.contains_key("http:chrono-instructions"));
        let Some(setup) = doc
            .get("tasks")
            .and_then(|tasks| tasks.get("harness-setup"))
            .and_then(toml::Value::as_table)
        else {
            return false;
        };
        if instruction_tool != instructions
            || setup.keys().any(|key| key != "run" && key != "description")
        {
            return false;
        }
        let commands: Vec<String> = match setup.get("run") {
            Some(toml::Value::String(one)) => vec![one.clone()],
            Some(toml::Value::Array(steps)) => {
                let Some(all) = steps
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                else {
                    return false;
                };
                all
            }
            _ => return false,
        };
        setup_sequence(&commands, instructions)
    }

    #[test]
    fn mise_installs_the_selected_tools_before_refreshing_instructions() {
        let original = mise_manifest();
        let selected = true;
        assert!(setup_matches(original, selected));
        assert!(!setup_matches(
            &original.replace(
                "sh .chrono-harness/install-tools.sh",
                "sh .chrono-harness/install-tools.sh --dry-run"
            ),
            selected
        ));
        assert!(!setup_matches(
            &original.replace(
                "[tasks.harness-setup]",
                "[tasks.harness-setup]\ndepends = ['other']"
            ),
            selected
        ));
        assert!(!setup_matches(
            &original.replace("sh .chrono-harness/setup-check-retention.sh", "true"),
            selected
        ));
        let install_only = "[tools]\n[tasks.harness-setup]\nrun = ['sh .chrono-harness/install-tools.sh', 'sh .chrono-harness/setup-check-retention.sh']";
        assert!(setup_matches(install_only, false));
        assert!(!setup_matches(install_only, true));
        let reverse = "[tools.'http:chrono-instructions']\n[tasks.harness-setup]\nrun=['mise exec -- chrono-instructions sync-rules --host-root .','sh .chrono-harness/install-tools.sh']";
        assert!(!setup_matches(reverse, true));
        let extra = "[tools]\n[tasks.harness-setup]\nrun = ['sh .chrono-harness/install-tools.sh', 'another-command']";
        assert!(!setup_matches(extra, false));
    }
}
