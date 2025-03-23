use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::Parser;
use semver::Version;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use tabwriter::TabWriter;

/// CLI tool for listing Helm charts from a chart repository (chartmuseum-style).
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The Helm chart repository source URL (e.g. https://bitnami-labs.github.io/sealed-secrets)
    #[arg(long)]
    source: String,

    /// (Optional) Filter by a specific chart name (case insensitive)
    #[arg(long)]
    chart: Option<String>,

    /// (Optional) Filter by chart type (case insensitive, e.g. "application" or "library")
    #[arg(long = "type")]
    chart_type: Option<String>,

    /// Disable the pager (enabled by default on outputs longer than 25 lines)
    #[arg(long)]
    no_pager: bool,
}

/// Represents the structure of the index.yaml file.
#[derive(Debug, Deserialize)]
struct IndexFile {
    entries: HashMap<String, Vec<ChartVersion>>,
    // Other fields (e.g. apiVersion, generated) are omitted for brevity.
}

/// Represents each chart version entry.
#[derive(Debug, Deserialize, Clone)]
struct ChartVersion {
    version: String,
    description: Option<String>,
    #[serde(rename = "appVersion")]
    app_version: Option<String>,
    #[serde(rename = "created")]
    created: Option<String>,
    #[serde(rename = "kubeVersion")]
    kube_version: Option<String>,
    #[serde(rename = "type")]
    chart_type: Option<String>,
}

/// Entry point.
fn main() -> Result<()> {
    let args = Args::parse();
    run(args)
}

/// Main application logic.
fn run(args: Args) -> Result<()> {
    let index = fetch_index(&args.source)?;

    // Collect the entries (chart names and their versions)
    let mut entries: Vec<(&String, &Vec<ChartVersion>)> = index.entries.iter().collect();

    // If the user specified a chart name, filter to that chart (case insensitive).
    if let Some(ref chart_name) = args.chart {
        entries.retain(|(name, _)| name.eq_ignore_ascii_case(chart_name));
        if entries.is_empty() {
            println!("No charts found for chart name: {}", chart_name);
            return Ok(());
        }
    }

    // Determine if the pager should be enabled.
    // Pager is enabled by default unless --no-pager is passed or the env vars are set.
    let disable_pager_env =
        std::env::var("HELM_LIST_CHARTS_NO_PAGER").is_ok() || std::env::var("NO_PAGER").is_ok();
    let pager_enabled = !args.no_pager && !disable_pager_env;

    // Buffer the output to a vector.
    let mut output_buf = Vec::new();
    {
        let mut tw = TabWriter::new(&mut output_buf);
        // Print header with columns: CHART, TYPE, VERSION, DESCRIPTION, APP VERSION, CREATED, KUBE VERSION.
        writeln!(
            tw,
            "CHART\tTYPE\tVERSION\tDESCRIPTION\tAPP VERSION\tCREATED\tKUBE VERSION"
        )?;

        // Process each chart entry.
        for (chart_name, versions) in entries {
            // If a type filter is specified, filter chart versions by chart_type (case insensitive).
            let filtered_versions: Vec<ChartVersion> =
                if let Some(ref filter_type) = args.chart_type {
                    versions
                        .iter()
                        .filter(|v| {
                            v.chart_type
                                .as_deref()
                                .map(|s| s.eq_ignore_ascii_case(filter_type))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect()
                } else {
                    versions.clone()
                };

            if filtered_versions.is_empty() {
                continue;
            }

            let lines = format_chart_versions(chart_name, &filtered_versions);
            for line in lines {
                writeln!(tw, "{}", line)?;
            }
        }
        tw.flush()
            .with_context(|| "Failed to flush tabwriter output")?;
    }

    // Convert the output to a string.
    let output_str = String::from_utf8(output_buf)
        .with_context(|| "Failed to convert output buffer to UTF-8")?;

    // Check if there's just one line: the titles. If so,
    // print a message and return early.
    if output_str.lines().count() == 1 {
        return Err(anyhow::anyhow!("No charts found."));
    }

    if pager_enabled && output_str.lines().count() >= 25 {
        // Determine pager program, defaulting to "less".
        let pager = std::env::var("PAGER").unwrap_or_else(|_| "less".to_string());
        let mut child = Command::new(pager)
            .stdin(Stdio::piped())
            .spawn()
            .with_context(|| "Failed to spawn pager process")?;
        {
            let child_stdin = child
                .stdin
                .as_mut()
                .context("Failed to open stdin for pager")?;
            child_stdin
                .write_all(output_str.as_bytes())
                .with_context(|| "Failed to write output to pager")?;
        }
        child
            .wait()
            .with_context(|| "Pager process encountered an error")?;
    } else {
        // No pager: write directly to stdout.
        io::stdout()
            .write_all(output_str.as_bytes())
            .with_context(|| "Failed to write output to stdout")?;
    }

    Ok(())
}

/// Fetches the index.yaml file from the given source URL and parses it.
fn fetch_index(source: &str) -> Result<IndexFile> {
    // Build the URL for the index.yaml (ensure there is no trailing slash)
    let url = format!("{}/index.yaml", source.trim_end_matches('/'));
    let response =
        reqwest::blocking::get(&url).with_context(|| format!("Failed to GET from URL: {}", url))?;
    let body = response
        .text()
        .with_context(|| "Failed to read response body as text")?;
    parse_index(&body)
}

/// Parses the given YAML content into an IndexFile.
fn parse_index(yaml: &str) -> Result<IndexFile> {
    serde_yaml::from_str(yaml).with_context(|| "Failed to parse YAML index file")
}

/// Formats the given `created` field value into a human-readable form.
/// If parsing fails or the field is not available, returns "<unspecified>".
fn format_created(created: &Option<String>) -> String {
    if let Some(s) = created {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            let local_dt: DateTime<Local> = dt.with_timezone(&Local);
            return local_dt.format("%b %-d, %Y %-I:%M %P").to_string();
        } else {
            return s.clone();
        }
    }
    "<unspecified>".to_string()
}

/// Returns a possibly shortened version of `text` with an ellipsis added if any cut occurred.
/// It avoids breaking words by cutting at the closest previous space.
/// `max_chars` specifies the maximum number of characters allowed before the ellipsis.
fn ellipsize(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    // Collect the first `max_chars` characters.
    let taken: String = text.chars().take(max_chars).collect();

    // Find the last space in the taken text.
    let trimmed = if let Some(pos) = taken.rfind(' ') {
        // Cut at the last space.
        taken[..pos].trim_end().to_string()
    } else {
        taken
    };

    format!("{}...", trimmed)
}

/// Formats the list of chart versions into tab-delimited lines.
/// The versions are sorted in descending order (newest version at the top).
/// Each row includes the columns: CHART, TYPE, VERSION, DESCRIPTION, APP VERSION, CREATED, KUBE VERSION.
fn format_chart_versions(chart_name: &str, versions: &[ChartVersion]) -> Vec<String> {
    // Clone and sort versions in descending order.
    let mut sorted_versions = versions.to_vec();
    sorted_versions.sort_by(|a, b| {
        // Attempt to parse the version strings as semantic versions.
        let ver_a = Version::parse(&a.version);
        let ver_b = Version::parse(&b.version);
        match (ver_a, ver_b) {
            (Ok(a_ver), Ok(b_ver)) => b_ver.cmp(&a_ver),
            _ => b.version.cmp(&a.version),
        }
    });

    let mut lines = Vec::new();
    for v in sorted_versions.iter() {
        let desc_excerpt = ellipsize(v.description.as_deref().unwrap_or(""), 50);
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            chart_name,
            v.chart_type.as_deref().unwrap_or("<unspecified>"),
            v.version,
            desc_excerpt,
            v.app_version.as_deref().unwrap_or("<unspecified>"),
            format_created(&v.created),
            v.kube_version.as_deref().unwrap_or("<unspecified>")
        ));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_chart_versions_full() {
        let chart_name = "test-chart";
        let versions = vec![
            ChartVersion {
                version: "1.0.0".to_string(),
                description: Some("Initial release".to_string()),
                app_version: Some("1.0".to_string()),
                created: Some("2025-02-13T12:42:23.967760696Z".to_string()),
                kube_version: Some(">= 1.19.0-0".to_string()),
                chart_type: Some("application".to_string()),
            },
            ChartVersion {
                version: "2.0.0".to_string(),
                description: Some("Second release with more features".to_string()),
                app_version: Some("2.0".to_string()),
                created: None,
                kube_version: None,
                chart_type: None,
            },
        ];

        let lines = format_chart_versions(chart_name, &versions);
        assert_eq!(lines.len(), 2);
        // Ensure that the newest version is first.
        assert!(lines[0].contains("2.0.0"));
        // Check for "<unspecified>" in missing fields.
        assert!(lines[0].contains("<unspecified>"));
    }

    // Tests for the ellipsize function.
    #[test]
    fn test_ellipsize_short_text() {
        let text = "Short text";
        let result = ellipsize(text, 50);
        assert_eq!(result, text.to_string());
    }

    #[test]
    fn test_ellipsize_exact_length() {
        let text = "Exact length text";
        let max_chars = text.chars().count();
        let result = ellipsize(text, max_chars);
        assert_eq!(result, text.to_string());
    }

    #[test]
    fn test_ellipsize_long_text_no_space() {
        let text = "abcdefghijk";
        // With no spaces, it should simply cut at max_chars and add ellipsis.
        let result = ellipsize(text, 5);
        assert_eq!(result, "abcde...");
    }

    #[test]
    fn test_ellipsize_long_text_with_space() {
        let text = "This is a longer text that should be cut off gracefully.";
        let result = ellipsize(text, 20);
        // Check that the result ends with an ellipsis.
        assert!(result.ends_with("..."));
        // Ensure it is shorter than the original text.
        assert!(result.len() < text.len());
    }

    // Test for parsing a YAML index.
    #[test]
    fn test_parse_index_valid() {
        let yaml = r#"
entries:
  test-chart:
    - version: "1.0.0"
      description: "Test version 1"
      appVersion: "1.0"
      created: "2025-02-13T12:42:23.967760696Z"
      kubeVersion: ">= 1.19.0-0"
      type: "application"
    - version: "2.0.0"
      description: "Test version 2"
      appVersion: "2.0"
      created: "2025-03-01T10:30:00.000000000Z"
      kubeVersion: ">= 1.20.0-0"
      type: "library"
"#;
        let index = parse_index(yaml).expect("Failed to parse YAML");
        assert!(index.entries.contains_key("test-chart"));
        let versions = &index.entries["test-chart"];
        assert_eq!(versions.len(), 2);
    }

    // Test for parsing an invalid YAML index.
    #[test]
    fn test_parse_index_invalid() {
        let invalid_yaml = "invalid_yaml: [";
        let result = parse_index(invalid_yaml);
        assert!(result.is_err(), "Expected an error for invalid YAML");
    }
}
