//! A pure rust library to handle camera raw files.
//! 
//! **quickraw** is a pure rust library to decode and renderer image from camera raw files.
//! 
//! ## Examples
//! #### Export thumbnail
//! ```no_run
//! use quickraw::Export;
//! 
//! let raw_data = std::fs::read("sample.ARW").unwrap();
//! let (thumbnail_data, orientation) = Export::export_thumbnail_data(&raw_data).unwrap();
//! 
//! // notice that this function is available on feature `image` only.
//! quickraw::Export::export_thumbnail_to_file("sample.ARW", "sample.thumbnail.jpg").unwrap();
//! ```
//! 
//! #### Get EXIF data
//! ```no_run
//! use quickraw::Export;
//! let info = Export::export_exif_info(Input::ByFile("sample.ARW")).unwrap();
//! 
//! // info is a `quickexif::ParsedInfo` type, for more info please check https://docs.rs/quickexif
//! let width = info.usize("width").unwrap();
//! ```
//! #### Export image
//! ```no_run
//! use quickraw::{data, DemosaicingMethod, Input, Output, Export, OutputType};
//! 
//! let demosaicing_method = DemosaicingMethod::Linear;
//! let color_space = data::XYZ2SRGB;
//! let gamma = data::GAMMA_SRGB;
//! let output_type = OutputType::Raw16;
//! let auto_crop = false;
//! let auto_rotate = false;
//! 
//! let export_job = Export::new(
//!     Input::ByFile("sample.ARW"),
//!     Output::new(
//!         demosaicing_method,
//!         color_space,
//!         gamma,
//!         output_type,
//!         auto_crop,
//!         auto_rotate,
//!     ),
//! ).unwrap();
//! 
//! let (image, width, height) = export_job.export_16bit_image();
//! 
//! // or you can also export an image with quality(only works when the output type is JPEG).
//! // notice that this function is available on feature `image` only.
//! export_job.export_image(92).unwrap();
//! ```

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

/// A flag to enable benchmark for several key processes.
pub const BENCH_FLAG: &str = "QUICKRAW_BENCH";

use thiserror::Error;
use std::fs;

pub mod data;

mod utility;

mod pass;
mod maker;
mod decode;
pub use decode::decode_file;
pub use decode::decode_buffer;
pub use decode::get_thumbnail;
pub use decode::Orientation;

#[cfg(feature = "wasm-bindgen")]
mod lib_wasm;
#[cfg(any(debug_assertions, not(feature = "wasm-bindgen")))]
mod lib_c;
#[cfg(any(debug_assertions, not(feature = "wasm-bindgen")))]
pub mod export;

const BIT_SHIFT: u32 = 13u32;

/// All the demosaicing method currently supported.
#[derive(Clone)]
pub enum DemosaicingMethod {
    None,
    SuperPixel,
    Linear,
}

/// Decides if the output should be 8bit or 16bit.
#[derive(Clone)]
pub enum OutputType {
    Raw8,
    Raw16,
    Image8(String),
    Image16(String),
}

/// Chooses the input from a file or a buffer.
pub enum Input<'a> {
    ByFile(&'a str),
    ByBuffer(Vec<u8>),
}

/// Contains options for image rendering.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Output {
    demosaicing_method: DemosaicingMethod,
    color_space: [f32; 9],
    gamma: [f32; 2],
    output_type: OutputType,
    auto_crop: bool,
    auto_rotate: bool,
}
impl Output {
    pub fn new(
        demosaicing_method: DemosaicingMethod,
        color_space: [f32; 9],
        gamma: [f32; 2],
        output_type: OutputType,
        auto_crop: bool,
        auto_rotate: bool,
    ) -> Output {
        Output {
            demosaicing_method,
            color_space,
            gamma,
            output_type,
            auto_crop,
            auto_rotate,
        }
    }
}

/// Errors of raw file reading.
#[derive(Error, Debug)]
pub enum RawFileReadingError {
    #[error("Exif parsing error.")]
    ExifParseError(#[from] quickexif::parser::Error),
    #[error("Exif parsed info error.")]
    ExifParseInfoError(#[from] quickexif::parsed_info::Error),
    #[error("Cannot read the raw file.")]
    DecodingError(#[from] maker::DecodingError),
    #[error("The file '{0}' is not existed.")]
    FileNotExisted(String),
    #[error("The metadata of file '{0}' cannot be read.")]
    FileMetadataReadingError(String),
    #[error("The content of file '{0}' cannot be read.")]
    FileContentReadingError(String),
    #[error("Cannot read Make info from this raw file.")]
    CannotReadMake,
    #[error("Cannot read Model info from this raw file.")]
    CannotReadModel,
    #[error("This raw file from maker: '{0}' is not supported yet.")]
    MakerIsNotSupportedYet(String),
    #[error("This raw file model: '{0}' is not supported yet.")]
    ModelIsNotSupportedYet(String),
}

pub struct Export;

impl Export {
    /// Export embedded thumbnail bytes from a raw buffer.
    pub fn export_thumbnail_data(buffer: &[u8]) -> Result<(Vec<u8>, Orientation), RawFileReadingError> {
        let (thumbnail, orientation) = decode::get_thumbnail(buffer)?;
        Ok((thumbnail.to_vec(), orientation))
    }

    /// Export embedded thumbnail bytes from a raw file.
    pub fn export_thumbnail_data_from_file(path: &str) -> Result<(Vec<u8>, Orientation), RawFileReadingError> {
        let buffer = fs::read(path)
            .map_err(|_| RawFileReadingError::FileContentReadingError(path.to_owned()))?;
        Self::export_thumbnail_data(&buffer)
    }

    /// Export embedded thumbnail to a file.
    pub fn export_thumbnail_to_file(raw_path: &str, out_path: &str) -> Result<(), RawFileReadingError> {
        let (thumbnail, _orientation) = Self::export_thumbnail_data_from_file(raw_path)?;
        fs::write(out_path, thumbnail)
            .map_err(|_| RawFileReadingError::FileContentReadingError(out_path.to_owned()))
    }
}
