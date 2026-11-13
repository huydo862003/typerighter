//! Read image dimensions by streaming only the bytes needed from a reader
//! Supports PNG, JPEG, GIF, WebP, and SVG

use std::io::Read;

pub struct ImageDimensions {
  pub width: u32,
  pub height: u32,
}

/// Read the minimum bytes needed to determine image dimensions
pub fn get_image_dimensions(reader: &mut impl Read) -> Option<ImageDimensions> {
  // 30 bytes covers PNG (24), GIF (10), WebP (30), and identifies JPEG
  let mut header = [0u8; 30];
  let bytes_read = read_exact_or_short(reader, &mut header);
  if bytes_read < 4 {
    return None;
  }
  let header = &header[..bytes_read];

  if let Some(dimension) = png(header) {
    return Some(dimension);
  }
  if let Some(dimension) = gif(header) {
    return Some(dimension);
  }
  if let Some(dimension) = webp(header) {
    return Some(dimension);
  }

  // JPEG needs to scan markers beyond the initial header
  if bytes_read >= 2 && header[0] == 0xFF && header[1] == 0xD8 {
    return jpeg_from_reader(header, reader);
  }

  // SVG needs more text - read up to 4KB to find the <svg> tag
  let mut buffer = Vec::with_capacity(4096);
  buffer.extend_from_slice(header);
  let mut remaining = vec![0u8; 4096 - bytes_read];
  let extra = read_exact_or_short(reader, &mut remaining);
  buffer.extend_from_slice(&remaining[..extra]);
  svg(&buffer)
}

fn read_exact_or_short(reader: &mut impl Read, buffer: &mut [u8]) -> usize {
  let mut total = 0;
  while total < buffer.len() {
    match reader.read(&mut buffer[total..]) {
      Ok(0) => break,
      Ok(n) => total += n,
      Err(_) => break,
    }
  }
  total
}

// PNG: IHDR at offset 16
fn png(data: &[u8]) -> Option<ImageDimensions> {
  if data.len() < 24 || data[0..4] != [0x89, 0x50, 0x4E, 0x47] {
    return None;
  }
  Some(ImageDimensions {
    width: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
    height: u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
  })
}

// GIF: logical screen descriptor at offset 6
fn gif(data: &[u8]) -> Option<ImageDimensions> {
  if data.len() < 10 || data[0..3] != [0x47, 0x49, 0x46] {
    return None;
  }
  Some(ImageDimensions {
    width: u16::from_le_bytes([data[6], data[7]]) as u32,
    height: u16::from_le_bytes([data[8], data[9]]) as u32,
  })
}

// WebP: VP8/VP8L/VP8X chunk
fn webp(data: &[u8]) -> Option<ImageDimensions> {
  if data.len() < 30 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
    return None;
  }
  match &data[12..16] {
    b"VP8 " => Some(ImageDimensions {
      width: (u16::from_le_bytes([data[26], data[27]]) & 0x3FFF) as u32,
      height: (u16::from_le_bytes([data[28], data[29]]) & 0x3FFF) as u32,
    }),
    b"VP8L" if data.len() >= 25 => {
      let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
      Some(ImageDimensions {
        width: (bits & 0x3FFF) + 1,
        height: ((bits >> 14) & 0x3FFF) + 1,
      })
    }
    b"VP8X" => Some(ImageDimensions {
      width: (data[24] as u32 | (data[25] as u32) << 8 | (data[26] as u32) << 16) + 1,
      height: (data[27] as u32 | (data[28] as u32) << 8 | (data[29] as u32) << 16) + 1,
    }),
    _ => None,
  }
}

// JPEG: scan marker segments for SOF, reading on demand from the reader
fn jpeg_from_reader(initial: &[u8], reader: &mut impl Read) -> Option<ImageDimensions> {
  let mut buffer = Vec::with_capacity(65536);
  buffer.extend_from_slice(initial);

  let mut offset = 2;
  loop {
    // Ensure we have enough data to read the next marker
    while buffer.len() < offset + 10 {
      let mut chunk = [0u8; 4096];
      match reader.read(&mut chunk) {
        Ok(0) => return None,
        Ok(n) => buffer.extend_from_slice(&chunk[..n]),
        Err(_) => return None,
      }
    }

    // Skip FF padding
    while offset < buffer.len() && buffer[offset] == 0xFF {
      offset += 1;
    }
    if offset >= buffer.len() {
      return None;
    }

    let marker = buffer[offset];
    let is_sof = (0xC0..=0xC3).contains(&marker)
      || (0xC5..=0xC7).contains(&marker)
      || (0xC9..=0xCB).contains(&marker)
      || (0xCD..=0xCF).contains(&marker);

    if is_sof {
      // Ensure we have the 8 bytes after the marker
      while buffer.len() < offset + 9 {
        let mut chunk = [0u8; 256];
        match reader.read(&mut chunk) {
          Ok(0) => return None,
          Ok(n) => buffer.extend_from_slice(&chunk[..n]),
          Err(_) => return None,
        }
      }
      return Some(ImageDimensions {
        height: u16::from_be_bytes([buffer[offset + 4], buffer[offset + 5]]) as u32,
        width: u16::from_be_bytes([buffer[offset + 6], buffer[offset + 7]]) as u32,
      });
    }

    // Skip non-SOF segment
    if offset + 2 >= buffer.len() {
      return None;
    }
    let segment_len = u16::from_be_bytes([buffer[offset + 1], buffer[offset + 2]]) as usize;
    if segment_len < 2 {
      return None;
    }
    offset += segment_len;

    // Safety: don't scan more than 256KB into a JPEG
    if offset > 262144 {
      return None;
    }
  }
}

// SVG: parse width/height or viewBox from <svg> tag
fn svg(data: &[u8]) -> Option<ImageDimensions> {
  let head = std::str::from_utf8(&data[..data.len().min(4096)]).ok()?;
  let svg_tag = head.split("<svg").nth(1)?.split('>').next()?;

  if let (Some(width), Some(height)) = (parse_attr(svg_tag, "width"), parse_attr(svg_tag, "height"))
  {
    return Some(ImageDimensions {
      width: width as u32,
      height: height as u32,
    });
  }

  let view_box_start = svg_tag.find("viewBox=\"")? + 9;
  let view_box_end = svg_tag[view_box_start..].find('"')? + view_box_start;
  let parts: Vec<f64> = svg_tag[view_box_start..view_box_end]
    .split_whitespace()
    .filter_map(|part| part.parse().ok())
    .collect();
  if parts.len() >= 4 {
    return Some(ImageDimensions {
      width: parts[2] as u32,
      height: parts[3] as u32,
    });
  }
  None
}

fn parse_attr(tag: &str, name: &str) -> Option<f64> {
  let needle = format!("{name}=\"");
  let start = tag.find(&needle)? + needle.len();
  tag[start..].split('"').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn png_dimensions() {
    let data = include_bytes!("../../../../tests/fixtures/1x1.png");
    let dimension = get_image_dimensions(&mut &data[..]).expect("should parse PNG");
    assert_eq!((dimension.width, dimension.height), (1, 1));
  }

  #[test]
  fn unknown_format_returns_none() {
    assert!(get_image_dimensions(&mut &b"not an image"[..]).is_none());
  }

  #[test]
  fn svg_with_viewbox() {
    let svg = br#"<svg viewBox="0 0 100 50" xmlns="http://www.w3.org/2000/svg"></svg>"#;
    let dimension = get_image_dimensions(&mut &svg[..]).expect("should parse SVG viewBox");
    assert_eq!((dimension.width, dimension.height), (100, 50));
  }

  #[test]
  fn svg_with_explicit_dimensions() {
    let svg = br#"<svg width="200" height="100" xmlns="http://www.w3.org/2000/svg"></svg>"#;
    let dimension = get_image_dimensions(&mut &svg[..]).expect("should parse SVG width/height");
    assert_eq!((dimension.width, dimension.height), (200, 100));
  }
}
