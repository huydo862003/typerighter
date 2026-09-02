// Read image dimensions from a file buffer by parsing format headers
// Supports PNG, JPEG, GIF, WebP, and SVG

export interface ImageDimensions {
  width: number;
  height: number;
}

export function getImageDimensions (
  buffer: Buffer,
): ImageDimensions | undefined {
  return (
    getPngDimensions(buffer)
    ?? getJpegDimensions(buffer)
    ?? getGifDimensions(buffer)
    ?? getWebpDimensions(buffer)
    ?? getSvgDimensions(buffer)
  );
}

// GIF (https://www.w3.org/Graphics/GIF/spec-gif89a.txt, section 18)
function getGifDimensions (buffer: Buffer): ImageDimensions | undefined {
  // GIF87a or GIF89a
  if (
    buffer.length < 10
    || buffer[0] !== 0x47
    || buffer[1] !== 0x49
    || buffer[2] !== 0x46
  )
    return undefined;

  return {
    width: buffer.readUInt16LE(6),
    height: buffer.readUInt16LE(8),
  };
}

// JPEG (https://www.w3.org/Graphics/JPEG/itu-t81.pdf, section B.2.2, table B.1)
function getJpegDimensions (buffer: Buffer): ImageDimensions | undefined {
  if (buffer.length < 2 || buffer[0] !== 0xff || buffer[1] !== 0xd8)
    return undefined;

  let offset = 2;

  while (offset + 1 < buffer.length) {
    // Skip 0xFF padding bytes
    while (offset < buffer.length && buffer[offset] === 0xff) offset++;

    if (buffer.length <= offset) return undefined;

    const marker = buffer[offset];

    // SOF markers (SOF0-SOF3, SOF5-SOF7, SOF9-SOF11, SOF13-SOF15)
    if (
      (0xc0 <= marker && marker <= 0xc3)
      || (0xc5 <= marker && marker <= 0xc7)
      || (0xc9 <= marker && marker <= 0xcb)
      || (0xcd <= marker && marker <= 0xcf)
    ) {
      if (buffer.length < offset + 8) return undefined;

      return {
        height: buffer.readUInt16BE(offset + 4),
        width: buffer.readUInt16BE(offset + 6),
      };
    }

    // Skip non-SOF marker segment
    if (buffer.length <= offset + 2) return undefined;

    const segmentLength = buffer.readUInt16BE(offset + 1);

    if (segmentLength < 2) return undefined;

    offset += segmentLength;
  }

  return undefined;
}

// PNG (https://www.w3.org/TR/png/#11IHDR)
function getPngDimensions (buffer: Buffer): ImageDimensions | undefined {
  // PNG signature: 137 80 78 71 13 10 26 10
  if (
    buffer.length < 24
    || buffer[0] !== 0x89
    || buffer[1] !== 0x50
    || buffer[2] !== 0x4e
    || buffer[3] !== 0x47
  )
    return undefined;

  return {
    width: buffer.readUInt32BE(16),
    height: buffer.readUInt32BE(20),
  };
}

// SVG (https://www.w3.org/TR/SVG2/struct.html#SVGElement)
function getSvgDimensions (buffer: Buffer): ImageDimensions | undefined {
  // Check for XML/SVG content in the first 4KB
  const head = buffer
    .subarray(0, Math.min(buffer.length, 4096))
    .toString('utf8');

  if (!head.includes('<svg')) return undefined;

  const svgTag = head.match(/<svg[^>]*>/);

  if (!svgTag) return undefined;

  const tag = svgTag[0];
  const widthMatch = tag.match(/\bwidth=["'](\d+(?:\.\d+)?)/);
  const heightMatch = tag.match(/\bheight=["'](\d+(?:\.\d+)?)/);

  if (widthMatch && heightMatch)
    return {
      width: Math.round(+widthMatch[1]),
      height: Math.round(+heightMatch[1]),
    };

  // Fall back to viewBox
  const viewBox = tag.match(
    /viewBox=["']\s*[\d.]+\s+[\d.]+\s+([\d.]+)\s+([\d.]+)/,
  );

  if (viewBox)
    return {
      width: Math.round(+viewBox[1]),
      height: Math.round(+viewBox[2]),
    };

  return undefined;
}

// WebP (https://developers.google.com/speed/webp/docs/riff_container)
function getWebpDimensions (buffer: Buffer): ImageDimensions | undefined {
  // RIFF....WEBP
  if (
    buffer.length < 30
    || buffer.toString('ascii', 0, 4) !== 'RIFF'
    || buffer.toString('ascii', 8, 12) !== 'WEBP'
  )
    return undefined;

  const format = buffer.toString('ascii', 12, 16);

  // VP8 lossy
  if (format === 'VP8 ' && 30 <= buffer.length) {
    // Frame tag starts at 20, dimensions at 26
    return {
      width: buffer.readUInt16LE(26) & 0x3fff,
      height: buffer.readUInt16LE(28) & 0x3fff,
    };
  }

  // VP8L lossless
  if (format === 'VP8L' && 25 <= buffer.length) {
    const bits = buffer.readUInt32LE(21);

    return {
      width: (bits & 0x3fff) + 1,
      height: ((bits >> 14) & 0x3fff) + 1,
    };
  }

  // VP8X extended
  if (format === 'VP8X' && 30 <= buffer.length) {
    return {
      width: (buffer[24] | (buffer[25] << 8) | (buffer[26] << 16)) + 1,
      height: (buffer[27] | (buffer[28] << 8) | (buffer[29] << 16)) + 1,
    };
  }

  return undefined;
}
