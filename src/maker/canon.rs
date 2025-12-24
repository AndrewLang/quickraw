use super::*;
use once_cell::sync::Lazy;

pub(super) struct General {
    info: quickexif::ParsedInfo,
}

pub(super) static THUMBNAIL_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
    quickexif::describe_rule!(tiff {
        0x0112 / orientation
        next {
            0x0201 / thumbnail
            0x0202 / thumbnail_len
        }
    })
});

pub(super) static IMAGE_RULE: Lazy<quickexif::ParsingRule> = Lazy::new(|| {
    quickexif::describe_rule!(tiff {
        0x0112 / orientation
        0x8769 {
            0xa002 / width
            0xa003 / height
        }
        next {
            0x0201 / thumbnail
            0x0202 / thumbnail_len
        }
    })
});

impl RawDecoder for General {
    fn new(info: quickexif::ParsedInfo) -> Self {
        General { info }
    }
    fn get_info(&self) -> &quickexif::ParsedInfo {
        &self.info
    }
    fn into_info(self) -> quickexif::ParsedInfo {
        self.info
    }
    fn get_crop(&self) -> Option<Crop> {
        None
    }
    fn get_cfa_pattern(&self) -> Result<CFAPattern, DecodingError> {
        Ok(CFAPattern::RGGB)
    }
    fn decode_with_preprocess(&self, _buffer: &[u8]) -> Result<Vec<u16>, DecodingError> {
        unimplemented!("Canon raw decoding is not implemented yet.")
    }
    fn get_thumbnail<'a>(&self, buffer: &'a [u8]) -> Result<&'a [u8], DecodingError> {
        // Prefer the Exif-provided preview when it looks like a displayable JPEG (APP0/APP1).
        if let Some(exif_jpeg) = jpeg_from_exif(buffer, &self.info) {
            return Ok(exif_jpeg);
        }

        // Fallback: scan for the largest displayable JPEG slice (skip raw lossless JPEG data).
        if let Some(scanned) = find_display_jpeg_slice(buffer) {
            return Ok(scanned);
        }

        Err(DecodingError::RawInfoError(
            quickexif::parsed_info::Error::FieldNotFound("thumbnail".into()),
        ))
    }
}

fn find_largest_jpeg_slice<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
    let mut start = 0usize;
    let mut best: Option<(usize, usize)> = None;
    while let Some(rel_soi) = buffer[start..].windows(3).position(|w| w == [0xff, 0xd8, 0xff]) {
        let soi = start + rel_soi;
        if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9]) {
            let end = soi + 3 + rel_eoi + 2;
            let len = end - soi;
            if best.map(|(_, b_len)| len > b_len).unwrap_or(true) {
                best = Some((soi, len));
            }
            start = end;
        } else {
            break;
        }
    }
    best.map(|(s, l)| &buffer[s..s + l])
}

fn jpeg_from_exif<'a>(buffer: &'a [u8], info: &quickexif::ParsedInfo) -> Option<&'a [u8]> {
    let offset = info.usize("thumbnail").ok()?;
    let len = info.usize("thumbnail_len").ok()?;
    if offset + len > buffer.len() || len < 4 {
        return None;
    }
    let slice = &buffer[offset..offset + len];
    if is_display_jpeg(slice) {
        Some(slice)
    } else {
        None
    }
}

fn is_valid_jpeg(slice: &[u8]) -> bool {
    if slice.len() < 4 || !slice.starts_with(&[0xff, 0xd8]) {
        return false;
    }
    slice
        .windows(2)
        .rev()
        .find(|w| w == &[0xff, 0xd9])
        .is_some()
}

fn is_display_jpeg(slice: &[u8]) -> bool {
    if !is_valid_jpeg(slice) {
        return false;
    }
    // Look for JFIF/EXIF APP markers near the start; avoid lossless RAW JPEG data that lacks them.
    slice
        .windows(4)
        .take(40)
        .any(|w| w == [0xff, 0xe0, b'J', b'F'] || w == [0xff, 0xe1, b'E', b'x'])
}

fn find_display_jpeg_slice<'a>(buffer: &'a [u8]) -> Option<&'a [u8]> {
    find_largest_jpeg_slice(buffer)
        .filter(|s| is_display_jpeg(s))
        .or_else(|| {
            // As a fallback, return the first valid JPEG slice, even without APP markers.
            let mut start = 0usize;
            while let Some(rel_soi) = buffer[start..].windows(3).position(|w| w == [0xff, 0xd8, 0xff]) {
                let soi = start + rel_soi;
                if let Some(rel_eoi) = buffer[soi + 3..].windows(2).position(|w| w == [0xff, 0xd9]) {
                    let end = soi + 3 + rel_eoi + 2;
                    let slice = &buffer[soi..end];
                    if is_valid_jpeg(slice) {
                        return Some(slice);
                    }
                    start = end;
                    continue;
                }
                break;
            }
            None
        })
}
