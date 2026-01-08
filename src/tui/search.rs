use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub channel: Option<String>,
    pub duration: Option<f64>,
    pub url: String,
}

#[derive(Deserialize)]
struct YtDlpPlaylist {
    entries: Vec<YtDlpEntry>,
}

#[derive(Deserialize)]
struct YtDlpEntry {
    id: String,
    title: String,
    channel: Option<String>,
    duration: Option<f64>,
    url: String,
}

pub fn search_youtube(query: &str, max_results: usize) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let search_term = format!("ytsearch{}:{}", max_results, query);

    let output = Command::new("yt-dlp")
        .args(["--flat-playlist", "-J", &search_term])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp search failed: {}", stderr).into());
    }

    let playlist: YtDlpPlaylist = serde_json::from_slice(&output.stdout)?;

    Ok(playlist
        .entries
        .into_iter()
        .map(|e| SearchResult {
            id: e.id,
            title: e.title,
            channel: e.channel,
            duration: e.duration,
            url: e.url,
        })
        .collect())
}
