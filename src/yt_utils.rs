use crate::result::Res;
use reqwest::Url;

fn extract_video_id(youtube_url: &str) -> Res<String> {
    let url = Url::parse(youtube_url)?;

    // Extract video ID from various YouTube URL formats
    if let Some(host) = url.host_str() {
        if host.contains("youtube.com") {
            // youtube.com/watch?v=VIDEO_ID format
            if let Some(video_id) = url
                .query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.to_string())
            {
                return Ok(video_id);
            }
        } else if host.contains("youtu.be") {
            // youtu.be/VIDEO_ID format
            if let Some(path) = url.path().strip_prefix('/') {
                return Ok(path.to_string());
            }
        }
    }

    Err("Could not extract YouTube video ID".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_id_for_watch_format() {
        let result = extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap();
        assert_eq!(result, "dQw4w9WgXcQ");
    }

    #[test]
    fn get_id_for_period_format() {
        let result = extract_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(result, "dQw4w9WgXcQ");
    }
}
