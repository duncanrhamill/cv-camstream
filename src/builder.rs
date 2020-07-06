//! # `CamStreamBuilder` implementation
//!
//! This module implements the builder for camera stream objects.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use std::path::{Path, PathBuf};

use serde_any;
use serde::de::DeserializeOwned;
use rscam::Config;

use crate::error::{Error, Result};
use crate::rectification::{RectifParams, StereoRectifParams};
use crate::camstream::StereoCamStream;
use image::ImageFormat;

// -----------------------------------------------------------------------------------------------
// TRAITS
// -----------------------------------------------------------------------------------------------

/// Provides common methods for enabling rectification of images by a stream builder.
pub trait Rectifiable: Sized {
    /// The parameters to be used, must be deserialisable.
    type Params: DeserializeOwned;

    fn rectif_params(self, params: Self::Params) -> Self;

    /// Load the rectification parameters from a file.
    ///
    /// The file type will be guessed at runtime, any file type supported by 
    /// [`serde_any`](https://docs.rs/serde_any/0.5.0/serde_any/) is supported, but it must be
    /// deserialisable into `Self::Params`.
    fn rectif_params_from_file<P: AsRef<Path>>(self, path: P) -> Result<Self> {
        // Check the file exitsts
        if !path.as_ref().exists() {
            return Err(Error::FileNotFound(path.as_ref().to_path_buf()));
        }

        // Load the parameters from the file, guessing which format they're in using serde_any
        let p = serde_any::from_file(path)
            .map_err(|e| Error::DeserialisationError(e))?;

        Ok(self.rectif_params(p))
    }
}

// -----------------------------------------------------------------------------------------------
// DATA STRUCTURES
// -----------------------------------------------------------------------------------------------

///
pub struct CamStreamBuilder {}

pub struct MonoStreamBuilder<'a> {
    path: Option<PathBuf>,

    rectif_params: Option<RectifParams>,

    config: Config<'a>
}

pub struct StereoStreamBuilder<'a> {
    left_path: Option<PathBuf>,
    right_path: Option<PathBuf>,

    rectif_params: Option<StereoRectifParams>,

    img_format: Option<ImageFormat>,

    left_config: Config<'a>,
    right_config: Config<'a>
}

// -----------------------------------------------------------------------------------------------
// IMPLEMENTATIONS
// -----------------------------------------------------------------------------------------------

impl CamStreamBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn mono<'a>(self) -> MonoStreamBuilder<'a> {
        MonoStreamBuilder { 
            path: None, 
            rectif_params: None,
            config: Config::default() 
        }
    }

    pub fn stereo<'a>(self) -> StereoStreamBuilder<'a> {
        StereoStreamBuilder {
            left_path: None,
            right_path: None,
            rectif_params: None,
            img_format: None,
            left_config: Config::default(),
            right_config: Config::default()
        }
    }
}

impl<'a> MonoStreamBuilder<'a> {
    /// Specify the path of the camera, i.e. the device path, such as `/dev/video1`
    ///
    /// # Returns
    /// - `self` if the path exists, `Err` otherwise
    pub fn path<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        if path.as_ref().exists() {
            self.path = Some(path.as_ref().to_path_buf());

            Ok(self)
        } else {
            Err(Error::FileNotFound(path.as_ref().to_path_buf()))
        }
    }

    /// Set the interval of the camera.
    ///
    /// V4L2 uses intervals rather than framerates, default value is `(1, 10)`.
    pub fn interval(mut self, interval: (u32, u32)) -> Self {
        self.config.interval = interval;

        self
    }

    /// Set the resolution of the camera.
    ///
    /// Default value is `(640, 480)`.
    pub fn resolution(mut self, resolution: (u32, u32)) -> Self {
        self.config.resolution = resolution;

        self
    }

    /// Set the format of the images.
    ///
    /// Uses the FourCC notation, default value is `b"YUYV"`.
    pub fn format(mut self, format: &'a [u8]) -> Self {
        self.config.format = format;

        self
    }

    /// Set the storage method for interlaced video.
    ///
    /// Possible values are those provided by the `rscam::FIELD_x` values, default is `FIELD_NONE`.
    pub fn field(mut self, field: u32) -> Self {
        self.config.field = field;

        self
    }

    /// Set the number of buffers in the queue for this camera.
    ///
    /// Default value is 2.
    pub fn num_buffers(mut self, num_buffers: u32) -> Self {
        self.config.nbuffers = num_buffers;

        self
    }
}

impl<'a> Rectifiable for MonoStreamBuilder<'a> {
    type Params = RectifParams;

    fn rectif_params(mut self, params: Self::Params) -> Self {
        self.rectif_params = Some(params);

        self
    }
}

impl<'a> StereoStreamBuilder<'a> {
    /// Specify the path of the left camera, i.e. the device path, such as `/dev/video1`
    ///
    /// # Returns
    /// - `self` if the path exists, `Err` otherwise
    pub fn left_path<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        if path.as_ref().exists() {
            self.left_path = Some(path.as_ref().to_path_buf());

            Ok(self)
        } else {
            Err(Error::FileNotFound(path.as_ref().to_path_buf()))
        }
    }

    /// Specify the path of the right camera, i.e. the device path, such as `/dev/video1`
    ///
    /// # Returns
    /// - `self` if the path exists, `Err` otherwise
    pub fn right_path<P: AsRef<Path>>(mut self, path: P) -> Result<Self> {
        if path.as_ref().exists() {
            self.right_path = Some(path.as_ref().to_path_buf());

            Ok(self)
        } else {
            Err(Error::FileNotFound(path.as_ref().to_path_buf()))
        }
    }

    /// Set the interval of both cameras.
    ///
    /// V4L2 uses intervals rather than framerates, default value is `(1, 10)`.
    pub fn interval(mut self, interval: (u32, u32)) -> Self {
        self.left_config.interval = interval;
        self.right_config.interval = interval;

        self
    }

    /// Set the resolution of both cameras.
    ///
    /// Default value is `(640, 480)`.
    pub fn resolution(mut self, resolution: (u32, u32)) -> Self {
        self.left_config.resolution = resolution;
        self.right_config.resolution = resolution;

        self
    }

    /// Set the format of the images.
    ///
    /// Uses the FourCC notation, default value is `b"YUYV"`.
    pub fn format(mut self, format: &'a [u8]) -> Result<Self> {
        self.img_format = format_from_fourcc(format);

        if self.img_format.is_none() {
            return Err(Error::ImageFormatError(String::from_utf8(format.into()).unwrap()));
        }

        self.left_config.format = format;
        self.right_config.format = format;

        Ok(self)
    }

    /// Set the storage method for interlaced video.
    ///
    /// Possible values are those provided by the `rscam::FIELD_x` values, default is `FIELD_NONE`.
    pub fn field(mut self, field: u32) -> Self {
        self.left_config.field = field;
        self.right_config.field = field;

        self
    }

    /// Set the number of buffers in the queue for both cameras.
    ///
    /// Default value is 2.
    pub fn num_buffers(mut self, num_buffers: u32) -> Self {
        self.left_config.nbuffers = num_buffers;
        self.right_config.nbuffers = num_buffers;

        self
    }

    /// Build the stereo camera stream object.
    ///
    /// This function can fail if the underlying V4L2 construction fails.
    pub fn build(self) -> Result<StereoCamStream> {
        // Confirm that required paths are present
        if self.left_path.is_none() || self.right_path.is_none() {
            return Err(Error::CamStreamBuildError(String::from("Missing camera path")));
        }

        // Build left camera
        let mut left_cam = rscam::Camera::new(self.left_path
            .unwrap()
            .to_str()
            .expect("Cannot convert left path to &str")
        ).map_err(|e| Error::CamStreamBuildError(format!("{}", e)))?;

        // Build right camera
        let mut right_cam = rscam::Camera::new(self.right_path
            .unwrap()
            .to_str()
            .expect("Cannot convert right path to &str")
        ).map_err(|e| Error::CamStreamBuildError(format!("{}", e)))?;

        // Start the cameras
        left_cam.start(&self.left_config).map_err(|e| Error::CamStartError(e))?;
        right_cam.start(&self.right_config).map_err(|e| Error::CamStartError(e))?;

        // Create new stream
        Ok(StereoCamStream::new(
            left_cam,
            right_cam,
            self.img_format.unwrap(),
            self.rectif_params
        ))
    }
}

impl<'a> Rectifiable for StereoStreamBuilder<'a> {
    type Params = StereoRectifParams;

    fn rectif_params(mut self, params: Self::Params) -> Self {
        self.rectif_params = Some(params);

        self
    }
}

// -----------------------------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// -----------------------------------------------------------------------------------------------

fn format_from_fourcc(format: &[u8]) -> Option<ImageFormat> {
    match format {
        b"MJPG" => Some(ImageFormat::Jpeg),
        _ => None
    }
}

// -----------------------------------------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    use super::*;

    /// Test that mono builders work corectly
    #[test]
    fn test_mono() {
        CamStreamBuilder::new()
            .mono()
            .path("/dev/video0")
            .expect("Cannot open /dev/video0")
            .rectif_params_from_file("res/video0_rectif_params.toml")
            .expect("Cannot load the rectification parameters");
    }

    /// Test that stereo builders work correctly
    #[test]
    fn test_stereo() {
        CamStreamBuilder::new()
            .stereo()
            .left_path("/dev/video2")
            .expect("Cannot open left camera")
            .right_path("/dev/video4")
            .expect("Cannot open right camera")
            .rectif_params_from_file("res/stereo_bench_rectif_params.toml")
            .expect("Cannot load recticiation parameters");
    }
}
