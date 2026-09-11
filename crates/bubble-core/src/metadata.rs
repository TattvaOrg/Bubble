use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use regex::Regex;
use serde_json::Value;

use crate::runtime_features::which;

pub fn has_exif_support() -> bool {
    which("exiftool")
}

pub fn has_taglib_support() -> bool {
    which("ffprobe")
}

pub fn has_video_support() -> bool {
    which("ffprobe")
}

pub fn has_pdf_support() -> bool {
    which("pdfinfo")
}

pub fn missing_deps_hint(mime_type: &str) -> Option<&'static str> {
    if mime_type.starts_with("image/") && !has_exif_support() {
        Some("Install exiftool for camera metadata (make/model, ISO, aperture, GPS)")
    } else if mime_type.starts_with("audio/") && !has_taglib_support() {
        Some("Install ffmpeg for audio metadata (artist, album, genre, duration)")
    } else if mime_type.starts_with("video/") && !has_video_support() {
        Some("Install ffmpeg for video metadata (duration, codec, resolution)")
    } else if mime_type == "application/pdf" && !has_pdf_support() {
        Some("Install poppler-utils for PDF metadata (author, title, page count)")
    } else {
        None
    }
}

pub fn format_duration(seconds: f64) -> Option<String> {
    if seconds <= 0.0 {
        return None;
    }
    let total = seconds as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        Some(format!("{h}:{m:02}:{s:02}"))
    } else {
        Some(format!("{m}:{s:02}"))
    }
}

pub fn extract_metadata(path: &Path, mime: &str) -> HashMap<String, String> {
    if mime.starts_with("image/") {
        extract_image(path)
    } else if mime.starts_with("audio/") {
        extract_audio(path)
    } else if mime.starts_with("video/") {
        extract_video(path)
    } else if mime == "application/pdf" {
        extract_pdf(path)
    } else {
        HashMap::new()
    }
}

fn extract_image(path: &Path) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    if !has_exif_support() {
        return meta;
    }

    let output = match Command::new("exiftool")
        .args([
            "-json",
            "-n",
            "-Make",
            "-Model",
            "-LensModel",
            "-DateTimeOriginal",
            "-ExposureTime",
            "-FNumber",
            "-ISO",
            "-FocalLength",
            "-Flash",
            "-WhiteBalance",
            "-MeteringMode",
            "-Software",
            "-GPSLatitude",
            "-GPSLatitudeRef",
            "-GPSLongitude",
            "-GPSLongitudeRef",
        ])
        .arg(path)
        .output()
    {
        Ok(out) => out,
        Err(_) => return meta,
    };

    if !output.status.success() {
        return meta;
    }

    if let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(&output.stdout) {
        if let Some(Value::Object(obj)) = arr.first() {
            let get_str = |k: &str| -> Option<String> {
                obj.get(k).and_then(|v| match v {
                    Value::String(s) => {
                        let t = s.trim();
                        if t.is_empty() { None } else { Some(t.to_string()) }
                    }
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
            };

            for &(label, key) in &[
                ("Camera make", "Make"),
                ("Camera model", "Model"),
                ("Lens model", "LensModel"),
                ("Date taken", "DateTimeOriginal"),
                ("ISO", "ISO"),
                ("Flash", "Flash"),
                ("White balance", "WhiteBalance"),
                ("Metering mode", "MeteringMode"),
                ("Software", "Software"),
            ] {
                if let Some(v) = get_str(key) {
                    meta.insert(label.to_string(), v);
                }
            }

            if let Some(Value::Number(n)) = obj.get("ExposureTime") {
                if let Some(v) = n.as_f64() {
                    if v > 0.0 {
                        let exp_str = if v >= 1.0 {
                            format!("{v:.1} s")
                        } else {
                            format!("1/{} s", (1.0 / v + 0.5) as u64)
                        };
                        meta.insert("Exposure".to_string(), exp_str);
                    }
                }
            }

            if let Some(Value::Number(n)) = obj.get("FNumber") {
                if let Some(v) = n.as_f64() {
                    meta.insert("Aperture".to_string(), format!("f/{v:.1}"));
                }
            }

            if let Some(Value::Number(n)) = obj.get("FocalLength") {
                if let Some(v) = n.as_f64() {
                    meta.insert("Focal length".to_string(), format!("{v:.0} mm"));
                }
            }

            if let (Some(lat), Some(lon)) = (get_str("GPSLatitude"), get_str("GPSLongitude")) {
                let lat_ref = get_str("GPSLatitudeRef").unwrap_or_default();
                let lon_ref = get_str("GPSLongitudeRef").unwrap_or_default();
                meta.insert("GPS".to_string(), format!("{lat} {lat_ref}, {lon} {lon_ref}"));
            }
        }
    }

    meta
}

fn run_ffprobe(path: &Path) -> Option<Value> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;

    if output.status.success() {
        serde_json::from_slice(&output.stdout).ok()
    } else {
        None
    }
}

fn first_stream_of_type<'a>(probe: &'a Value, kind: &str) -> Option<&'a Value> {
    probe
        .get("streams")
        .and_then(|v| v.as_array())?
        .iter()
        .find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some(kind))
}

fn extract_audio(path: &Path) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    if !has_taglib_support() {
        return meta;
    }

    let probe = match run_ffprobe(path) {
        Some(v) => v,
        None => return meta,
    };

    let fmt = probe.get("format");
    let tags = fmt.and_then(|f| f.get("tags")).and_then(|t| t.as_object());
    let audio = first_stream_of_type(&probe, "audio");

    if let Some(tags_map) = tags {
        let tag_value = |wanted_keys: &[&str]| -> Option<String> {
            for (k, v) in tags_map {
                let k_lower = k.to_lowercase();
                for &wanted in wanted_keys {
                    if k_lower == wanted {
                        if let Some(s) = v.as_str() {
                            let t = s.trim();
                            if !t.is_empty() {
                                return Some(t.to_string());
                            }
                        }
                    }
                }
            }
            None
        };

        let mut put_tag = |label: &str, keys: &[&str]| {
            if let Some(v) = tag_value(keys) {
                meta.insert(label.to_string(), v);
            }
        };

        put_tag("Title", &["title"]);
        put_tag("Artist", &["artist", "author"]);
        put_tag("Album", &["album"]);
        put_tag("Genre", &["genre"]);
        put_tag("Year", &["date", "year"]);
        put_tag("Track", &["track"]);
        put_tag("Comment", &["comment"]);
    }

    if let Some(fmt_obj) = fmt {
        if let Some(dur_str) = fmt_obj.get("duration").and_then(|d| d.as_str()) {
            if let Ok(dur) = dur_str.parse::<f64>() {
                if let Some(formatted) = format_duration(dur) {
                    meta.insert("Duration".to_string(), formatted);
                }
            }
        }
        if let Some(br_str) = fmt_obj.get("bit_rate").and_then(|b| b.as_str()) {
            if let Ok(bitrate) = br_str.parse::<u64>() {
                if bitrate > 0 {
                    meta.insert("Bitrate".to_string(), format!("{} kbps", bitrate / 1000));
                }
            }
        }
    }

    if let Some(audio_obj) = audio {
        if let Some(sr_str) = audio_obj.get("sample_rate").and_then(|s| s.as_str()) {
            meta.insert("Sample rate".to_string(), format!("{sr_str} Hz"));
        }
        if let Some(channels) = audio_obj.get("channels").and_then(|c| c.as_i64()) {
            let ch_str = match channels {
                1 => "Mono",
                2 => "Stereo",
                _ => "",
            };
            if !ch_str.is_empty() {
                meta.insert("Channels".to_string(), ch_str.to_string());
            }
        }
    }

    meta
}

fn extract_video(path: &Path) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    if !has_video_support() {
        return meta;
    }

    let probe = match run_ffprobe(path) {
        Some(v) => v,
        None => return meta,
    };

    let fmt = probe.get("format");
    if let Some(fmt_obj) = fmt {
        if let Some(dur_str) = fmt_obj.get("duration").and_then(|d| d.as_str()) {
            if let Ok(dur) = dur_str.parse::<f64>() {
                if let Some(formatted) = format_duration(dur) {
                    meta.insert("Duration".to_string(), formatted);
                }
            }
        }
        if let Some(br_str) = fmt_obj.get("bit_rate").and_then(|b| b.as_str()) {
            if let Ok(bitrate) = br_str.parse::<u64>() {
                if bitrate > 0 {
                    let mbps = bitrate as f64 / 1e6;
                    let br_formatted = if mbps >= 1.0 {
                        format!("{mbps:.1} Mbps")
                    } else {
                        format!("{} kbps", bitrate / 1000)
                    };
                    meta.insert("Bitrate".to_string(), br_formatted);
                }
            }
        }
    }

    if let Some(video) = first_stream_of_type(&probe, "video") {
        if let Some(codec) = video.get("codec_name").and_then(|c| c.as_str()) {
            meta.insert("Video codec".to_string(), codec.to_string());
        }
        let w = video.get("width").and_then(|w| w.as_i64());
        let h = video.get("height").and_then(|h| h.as_i64());
        if let (Some(w_val), Some(h_val)) = (w, h) {
            if w_val > 0 && h_val > 0 {
                meta.insert("Resolution".to_string(), format!("{w_val} x {h_val}"));
            }
        }
        if let Some(fps_str) = video.get("avg_frame_rate").and_then(|f| f.as_str()) {
            if let Some((num_s, den_s)) = fps_str.split_once('/') {
                if let (Ok(num), Ok(den)) = (num_s.parse::<f64>(), den_s.parse::<f64>()) {
                    if num > 0.0 && den > 0.0 {
                        meta.insert("Frame rate".to_string(), format!("{:.1} fps", num / den));
                    }
                }
            }
        }
    }

    if let Some(audio) = first_stream_of_type(&probe, "audio") {
        if let Some(codec) = audio.get("codec_name").and_then(|c| c.as_str()) {
            meta.insert("Audio codec".to_string(), codec.to_string());
        }
        if let Some(sr_str) = audio.get("sample_rate").and_then(|s| s.as_str()) {
            meta.insert("Sample rate".to_string(), format!("{sr_str} Hz"));
        }
        if let Some(channels) = audio.get("channels").and_then(|c| c.as_i64()) {
            let ch_str = match channels {
                1 => "Mono".to_string(),
                2 => "Stereo".to_string(),
                n if n > 2 => format!("{}.{}", n - 1, if n > 5 { 1 } else { 0 }),
                _ => String::new(),
            };
            if !ch_str.is_empty() {
                meta.insert("Audio channels".to_string(), ch_str);
            }
        }
    }

    meta
}

fn extract_pdf(path: &Path) -> HashMap<String, String> {
    let mut meta = HashMap::new();
    if !has_pdf_support() {
        return meta;
    }

    let output = match Command::new("pdfinfo").arg(path).output() {
        Ok(out) => out,
        Err(_) => return meta,
    };

    if !output.status.success() {
        return meta;
    }

    let out_str = String::from_utf8_lossy(&output.stdout);
    let mut kv = HashMap::new();
    for line in out_str.lines() {
        if let Some((k, v)) = line.split_once(':') {
            kv.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let mut put = |label: &str, key: &str| {
        if let Some(v) = kv.get(key) {
            if !v.is_empty() {
                meta.insert(label.to_string(), v.clone());
            }
        }
    };

    put("Title", "Title");
    put("Author", "Author");
    put("Subject", "Subject");
    put("Creator", "Creator");
    put("Producer", "Producer");
    put("Created", "CreationDate");
    put("Modified", "ModDate");
    put("Pages", "Pages");
    put("PDF version", "PDF version");

    if let Some(raw_size) = kv.get("Page size") {
        let re = Regex::new(r"([0-9.]+)\s*x\s*([0-9.]+)\s*pts").unwrap();
        if let Some(caps) = re.captures(raw_size) {
            if let (Ok(w), Ok(h)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
                let w_mm = w * 25.4 / 72.0;
                let h_mm = h * 25.4 / 72.0;
                meta.insert("Page size".to_string(), format!("{w_mm:.0} x {h_mm:.0} mm"));
            }
        }
    }

    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(65.0), Some("1:05".to_string()));
        assert_eq!(format_duration(3665.0), Some("1:01:05".to_string()));
        assert_eq!(format_duration(0.0), None);
    }
}
