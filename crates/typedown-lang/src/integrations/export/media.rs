//! Centralized media URL classification for `![alt](url "title")` syntax.
//!
//! Determines whether a URL should render as an image, a known service embed,
//! a GitHub code snippet, or a generic iframe.

use std::fmt::Write;
use std::path::Path;

use crate::db::utils::{is_external_url, percent_encode};

const IMAGE_EXTENSIONS: &[&str] = &[
  "png", "jpg", "jpeg", "gif", "svg", "webp", "avif", "ico", "bmp", "tiff",
];

/// Domains that always serve images regardless of file extension
const IMAGE_DOMAINS: &[&str] = &["img.shields.io", "badge.fury.io", "img.buymeacoffee.com"];

/// How a media URL should be rendered
pub enum MediaKind {
  /// Standard `<img>` tag
  Image,
  /// GitHub file permalink or gist (client-side embed via emgithub)
  GithubCode,
  /// Known service with a transformed embed URL
  ServiceEmbed(ServiceEmbed),
  /// Unknown external URL - render as a sandboxed iframe
  GenericIframe,
}

pub struct ServiceEmbed {
  pub embed_url: String,
  pub allow: &'static str,
  pub style: &'static str,
  pub sandbox: &'static str,
}

impl ServiceEmbed {
  pub fn generic_iframe(url: &str) -> Self {
    Self {
      embed_url: url.to_string(),
      allow: "",
      style: "",
      sandbox: "allow-scripts allow-same-origin",
    }
  }
}

/// Classify a URL into the appropriate media rendering strategy.
///
/// Priority:
/// 1. Image file extension or known image domain -> Image
/// 2. GitHub file/gist URL -> GithubCode
/// 3. Known embeddable service -> ServiceEmbed
/// 4. External URL without image extension -> GenericIframe
/// 5. Relative URL without image extension -> Image (assume local asset)
pub fn classify_media_url(url: &str) -> MediaKind {
  let url = url.trim();

  if is_image_url(url) {
    return MediaKind::Image;
  }

  if is_github_code_url(url) {
    return MediaKind::GithubCode;
  }

  if let Some(embed) = try_service_embed(url) {
    return MediaKind::ServiceEmbed(embed);
  }

  if is_external_url(url) {
    return MediaKind::GenericIframe;
  }

  MediaKind::Image
}

/// Extract the hostname from a URL (without port)
fn extract_host(url: &str) -> Option<&str> {
  let after_scheme = url
    .strip_prefix("https://")
    .or(url.strip_prefix("http://"))?;
  Some(
    after_scheme
      .split('/')
      .next()
      .unwrap_or(after_scheme)
      .split(':')
      .next()
      .unwrap_or(""),
  )
}

/// Extract the path component from a URL
fn extract_path(url: &str) -> &str {
  let after_scheme = url
    .strip_prefix("https://")
    .or(url.strip_prefix("http://"))
    .unwrap_or(url);
  let after_host = after_scheme
    .find('/')
    .map_or("", |index| &after_scheme[index..]);
  after_host.split(['?', '#']).next().unwrap_or(after_host)
}

fn is_image_url(url: &str) -> bool {
  if has_image_extension(url) {
    return true;
  }
  if let Some(host) = extract_host(url) {
    return IMAGE_DOMAINS.contains(&host);
  }
  false
}

fn has_image_extension(url: &str) -> bool {
  let path = extract_path(url);
  Path::new(path)
    .extension()
    .and_then(|extension| extension.to_str())
    .is_some_and(|extension| {
      IMAGE_EXTENSIONS
        .iter()
        .any(|image_extension| extension.eq_ignore_ascii_case(image_extension))
    })
}

fn is_github_code_url(url: &str) -> bool {
  let Some(host) = extract_host(url) else {
    return false;
  };
  let path = extract_path(url);
  if host == "github.com" {
    let segments: Vec<&str> = path.trim_start_matches('/').splitn(4, '/').collect();
    return segments.len() >= 3 && segments[2] == "blob";
  }
  host == "gist.github.com"
}

const YOUTUBE_ALLOW: &str =
  "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture";
const VIMEO_ALLOW: &str = "autoplay; fullscreen; picture-in-picture";
const SPOTIFY_ALLOW: &str =
  "autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture";
const APPLE_MUSIC_ALLOW: &str = "autoplay; encrypted-media; fullscreen";

fn try_service_embed(url: &str) -> Option<ServiceEmbed> {
  let host = extract_host(url)?;
  let path = extract_path(url);

  match host {
    "www.youtube.com" | "youtube.com" => {
      if path.starts_with("/embed/") {
        // Already an embed URL, pass through
        Some(ServiceEmbed {
          embed_url: url.to_string(),
          allow: YOUTUBE_ALLOW,
          style: "aspect-ratio: 16 / 9",
          sandbox: "",
        })
      } else {
        let id = extract_youtube_id(url)?;
        Some(ServiceEmbed {
          embed_url: format!("https://www.youtube.com/embed/{id}"),
          allow: YOUTUBE_ALLOW,
          style: "aspect-ratio: 16 / 9",
          sandbox: "",
        })
      }
    }
    "youtu.be" => {
      let id = path
        .trim_start_matches('/')
        .split(['?', '&', '#'])
        .next()
        .filter(|s| !s.is_empty())?;
      Some(ServiceEmbed {
        embed_url: format!("https://www.youtube.com/embed/{id}"),
        allow: YOUTUBE_ALLOW,
        style: "aspect-ratio: 16 / 9",
        sandbox: "",
      })
    }
    "open.spotify.com" => {
      let path = path.trim_start_matches('/');
      let is_track = path.starts_with("track/") || path.starts_with("embed/track/");
      let embed_url = if path.starts_with("embed/") {
        url.to_string()
      } else {
        format!("https://open.spotify.com/embed/{path}")
      };
      Some(ServiceEmbed {
        embed_url,
        allow: SPOTIFY_ALLOW,
        style: if is_track {
          "height: 152px"
        } else {
          "height: 352px"
        },
        sandbox: "",
      })
    }
    "embed.music.apple.com" => Some(ServiceEmbed {
      embed_url: url.to_string(),
      allow: APPLE_MUSIC_ALLOW,
      style: "height: 175px",
      sandbox: "",
    }),
    "music.apple.com" => Some(ServiceEmbed {
      embed_url: url.replacen("music.apple.com", "embed.music.apple.com", 1),
      allow: APPLE_MUSIC_ALLOW,
      style: "height: 175px",
      sandbox: "",
    }),
    "player.vimeo.com" => Some(ServiceEmbed {
      embed_url: url.to_string(),
      allow: VIMEO_ALLOW,
      style: "aspect-ratio: 16 / 9",
      sandbox: "",
    }),
    "vimeo.com" => {
      let id = path.trim_start_matches('/').split(['/', '?', '#']).next()?;
      if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
        Some(ServiceEmbed {
          embed_url: format!("https://player.vimeo.com/video/{id}"),
          allow: VIMEO_ALLOW,
          style: "aspect-ratio: 16 / 9",
          sandbox: "",
        })
      } else {
        None
      }
    }
    "codepen.io" if path.contains("/pen/") => {
      let embed_url = url.replacen("/pen/", "/embed/", 1);
      Some(ServiceEmbed {
        embed_url,
        allow: "",
        style: "",
        sandbox: "",
      })
    }
    "codesandbox.io" if path.starts_with("/s/") => {
      let id = &path[3..];
      Some(ServiceEmbed {
        embed_url: format!("https://codesandbox.io/embed/{id}"),
        allow: "accelerometer; camera; encrypted-media; geolocation; gyroscope; microphone",
        style: "aspect-ratio: 4 / 3",
        sandbox: "",
      })
    }
    "www.figma.com" if path.starts_with("/file/") || path.starts_with("/design/") => {
      let encoded = percent_encode(url);
      Some(ServiceEmbed {
        embed_url: format!("https://www.figma.com/embed?embed_host=share&url={encoded}"),
        allow: "fullscreen",
        style: "aspect-ratio: 16 / 9",
        sandbox: "",
      })
    }
    "excalidraw.com" => Some(ServiceEmbed {
      embed_url: url.to_string(),
      allow: "",
      style: "aspect-ratio: 4 / 3",
      sandbox: "",
    }),
    _ => None,
  }
}

fn extract_youtube_id(url: &str) -> Option<&str> {
  let path = extract_path(url);
  if path != "/watch" {
    return None;
  }
  let query = url.split('?').nth(1)?;
  query
    .split('&')
    .find_map(|param| param.strip_prefix("v="))
    .map(|id| id.split('&').next().unwrap_or(id))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn assert_image(url: &str) {
    assert!(
      matches!(classify_media_url(url), MediaKind::Image),
      "{url} should be Image"
    );
  }

  fn assert_github(url: &str) {
    assert!(
      matches!(classify_media_url(url), MediaKind::GithubCode),
      "{url} should be GithubCode"
    );
  }

  fn assert_iframe(url: &str) {
    assert!(
      matches!(classify_media_url(url), MediaKind::GenericIframe),
      "{url} should be GenericIframe"
    );
  }

  fn assert_embed(url: &str, expected_embed_url: &str) {
    match classify_media_url(url) {
      MediaKind::ServiceEmbed(e) => {
        assert_eq!(
          e.embed_url, expected_embed_url,
          "embed URL mismatch for {url}"
        );
      }
      _ => panic!("{url} should be ServiceEmbed"),
    }
  }

  #[test]
  fn image_by_extension() {
    assert_image("photo.png");
    assert_image("photo.JPG");
    assert_image("/assets/icon.svg");
    assert_image("img.webp?w=100");
    assert_image("https://example.com/photo.png");
  }

  #[test]
  fn image_by_domain() {
    assert_image("https://img.shields.io/badge/Email-blue");
    assert_image("https://badge.fury.io/js/typerighter");
    assert_image("https://img.buymeacoffee.com/button.png");
  }

  #[test]
  fn relative_urls_are_images() {
    assert_image("assets/diagram");
    assert_image("/files/doc");
    assert_image("_assets/photo");
  }

  #[test]
  fn github_code() {
    assert_github("https://github.com/user/repo/blob/main/src/lib.rs#L10-L20");
    assert_github("https://gist.github.com/user/abc123");
  }

  #[test]
  fn youtube() {
    assert_embed(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
      "https://www.youtube.com/embed/dQw4w9WgXcQ",
    );
    assert_embed(
      "https://youtube.com/watch?v=abc123",
      "https://www.youtube.com/embed/abc123",
    );
    assert_embed(
      "https://youtu.be/dQw4w9WgXcQ",
      "https://www.youtube.com/embed/dQw4w9WgXcQ",
    );
  }

  #[test]
  fn spotify() {
    assert_embed(
      "https://open.spotify.com/track/abc123",
      "https://open.spotify.com/embed/track/abc123",
    );
    assert_embed(
      "https://open.spotify.com/album/xyz",
      "https://open.spotify.com/embed/album/xyz",
    );
    assert_embed(
      "https://open.spotify.com/playlist/my-list",
      "https://open.spotify.com/embed/playlist/my-list",
    );
  }

  #[test]
  fn vimeo() {
    assert_embed(
      "https://vimeo.com/123456",
      "https://player.vimeo.com/video/123456",
    );
  }

  #[test]
  fn codepen() {
    assert_embed(
      "https://codepen.io/user/pen/abcdef",
      "https://codepen.io/user/embed/abcdef",
    );
  }

  #[test]
  fn codesandbox() {
    assert_embed(
      "https://codesandbox.io/s/my-sandbox",
      "https://codesandbox.io/embed/my-sandbox",
    );
  }

  #[test]
  fn direct_embed_urls_pass_through() {
    assert_embed(
      "https://www.youtube.com/embed/dQw4w9WgXcQ",
      "https://www.youtube.com/embed/dQw4w9WgXcQ",
    );
    assert_embed(
      "https://open.spotify.com/embed/track/abc123",
      "https://open.spotify.com/embed/track/abc123",
    );
    assert_embed(
      "https://player.vimeo.com/video/123456",
      "https://player.vimeo.com/video/123456",
    );
    assert_embed(
      "https://embed.music.apple.com/us/album/foo/123",
      "https://embed.music.apple.com/us/album/foo/123",
    );
  }

  #[test]
  fn generic_iframe() {
    assert_iframe("https://example.com/some-page");
    assert_iframe("https://docs.google.com/spreadsheets/d/abc");
    // dbdiagram embed URLs fall through to generic iframe
    assert_iframe("https://dbdiagram.io/d/my-diagram-id");
    assert_iframe("https://dbdiagram.io/e/my-diagram-id");
    assert_iframe("https://dbdiagram.io/embed#c=SGVsbG8gV29ybGQ%3D");
  }
}
