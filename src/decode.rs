use super::*;
use std::{fs::File, io::Read};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
pub enum CFAPattern {
    RGGB,
    GRBG,
    GBRG,
    BGGR,
    XTrans0, // RBGBRG
    XTrans1, // GGRGGB
}

pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct DecodedImage {
    pub cfa_pattern: CFAPattern,
    pub width: usize,
    pub height: usize,
    pub crop: Option<Crop>,
    pub orientation: Orientation,
    pub image: Vec<u16>,
    pub white_balance: [i32; 3],
    pub cam_matrix: [f32; 9],
    pub parsed_info: quickexif::ParsedInfo,
}

pub enum Orientation {
    Horizontal = 0,
    Rotate90 = 90,
    Rotate180 = 180,
    Rotate270 = 270,
}

pub(super) fn get_buffer_from_file(path: &str) -> Result<Vec<u8>, RawFileReadingError> {
    let mut f =
        File::open(path).map_err(|_| RawFileReadingError::FileNotExisted(path.to_owned()))?;
    let len = f
        .metadata()
        .map_err(|_| RawFileReadingError::FileMetadataReadingError(path.to_owned()))?
        .len() as usize;
    let mut buffer = vec![0u8; len];
    f.read(&mut buffer)
        .map_err(|_| RawFileReadingError::FileContentReadingError(path.to_owned()))?;

    Ok(buffer)
}
fn prepare_buffer(mut buffer: Vec<u8>) -> Vec<u8> {
    buffer.extend([0u8; 16]); // + 16 is for BitPumpMSB fix

    fuji_buffer_fix(buffer)
}
fn fuji_buffer_fix(buffer: Vec<u8>) -> Vec<u8> {
    if buffer[..4] == [0x46, 0x55, 0x4a, 0x49] {
        buffer[148..].to_vec()
    } else {
        buffer
    }
}
fn fuji_buffer_slice_fix(buffer: &[u8]) -> &[u8] {
    if buffer[..4] == [0x46, 0x55, 0x4a, 0x49] {
        &buffer[148..]
    } else {
        buffer
    }
}

fn largest_jpeg_slice(buffer: &[u8]) -> Option<&[u8]> {
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

fn try_cr3_thumbnail(buffer: &[u8]) -> Option<(&[u8], Orientation)> {
    let is_cr3 = buffer.get(4..12).map(|b| b == b"ftypcrx ").unwrap_or(false);
    if !is_cr3 {
        return None;
    }
    let jpeg = largest_jpeg_slice(buffer)?;
    Some((jpeg, Orientation::Horizontal))
}
fn is_tiff_header(bytes: &[u8]) -> bool {
    bytes == [0x49, 0x49, 0x2a, 0x00] || bytes == [0x4d, 0x4d, 0x00, 0x2a]
}

fn canon_cr3_exif_slice(buffer: &[u8]) -> Option<&[u8]> {
    const EXIF_HEADER: &[u8] = b"Exif\0\0";

    // Prefer the Exif marker if present.
    if let Some(pos) = buffer
        .windows(EXIF_HEADER.len())
        .position(|window| window == EXIF_HEADER)
    {
        let after_exif = pos + EXIF_HEADER.len();
        if let Some(header) = buffer.get(after_exif..after_exif + 4) {
            if is_tiff_header(header) {
                return buffer.get(after_exif..);
            }
        }
        // If Exif is present but not immediately followed by TIFF header, scan forward for it.
        if let Some(rel) = buffer[after_exif..]
            .windows(4)
            .position(|w| is_tiff_header(w))
        {
            let start = after_exif + rel;
            return buffer.get(start..);
        }
    }

    // Fallback: scan the entire buffer for a TIFF header.
    if let Some(pos) = buffer.windows(4).position(|w| is_tiff_header(w)) {
        return buffer.get(pos..);
    }

    None
}

fn parse_basic_info_with_fallback<'a>(
    buffer: &'a [u8],
) -> Result<(quickexif::ParsedInfo, &'a [u8]), RawFileReadingError> {
    let buffer = fuji_buffer_slice_fix(buffer);
    let rule = &utility::BASIC_INFO_RULE;
    match quickexif::parse(buffer, rule) {
        Ok(info) => Ok((info, buffer)),
        Err(e) => {
            if let Some(exif_buffer) = canon_cr3_exif_slice(buffer) {
                Ok((quickexif::parse(exif_buffer, rule)?, exif_buffer))
            } else {
                Err(e.into())
            }
        }
    }
}

/// Gets `RawImage` from a file
#[cfg_attr(not(feature = "wasm-bindgen"), fn_util::bench(decoding))]
pub fn decode_file(path: &str) -> Result<DecodedImage, RawFileReadingError> {
    let buffer = get_buffer_from_file(path)?;
    decode_buffer(buffer)
}

/// Gets `RawImage` from a buffer
#[inline(always)]
pub fn decode_buffer(buffer: Vec<u8>) -> Result<DecodedImage, RawFileReadingError> {
    let buffer = prepare_buffer(buffer);

    let rule = &utility::BASIC_INFO_RULE;
    let decoder_select_info = quickexif::parse(&buffer, rule)?;

    let decoded_image = maker::selector::select_and_decode(buffer.as_slice(), decoder_select_info)?;

    Ok(decoded_image)
}

pub(super) fn get_exif_info(buffer: &[u8]) -> Result<quickexif::ParsedInfo, RawFileReadingError> {
    let (decoder_select_info, buffer) = parse_basic_info_with_fallback(buffer)?;
    let result = maker::selector::select_and_decode_exif_info(buffer, decoder_select_info)?;
    Ok(result)
}

pub fn get_thumbnail(buffer: &[u8]) -> Result<(&[u8], Orientation), RawFileReadingError> {
    if let Some(result) = try_cr3_thumbnail(buffer) {
        return Ok(result);
    }

    match parse_basic_info_with_fallback(buffer) {
        Ok((decoder_select_info, buffer)) => {
            let result = maker::selector::select_and_decode_thumbnail(buffer, decoder_select_info)?;
            Ok(result)
        }
        Err(e) => {
            if let Some(jpeg) = largest_jpeg_slice(buffer) {
                Ok((jpeg, Orientation::Horizontal))
            } else {
                Err(e)
            }
        }
    }
}
