//! # `cv_camstream` Error module
//!
//! Provides abstractions over errors which can occur during this crate's use.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use std::path::PathBuf;

use serde_any;
use thiserror;

// -----------------------------------------------------------------------------------------------
// ENUMERATIONS
// -----------------------------------------------------------------------------------------------

/// Result type used by faillible functions inside the `cv_camstream` crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents errors which can occur during use of the `cv_camstream` crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Cannot find file at {0:?}")]
    FileNotFound(PathBuf),

    #[error("Error deserialising data: {0}")]
    DeserialisationError(serde_any::Error),

    #[error(
        "Cannot convert RectifParams to CameraIntrisics struct as this would discard the \
        RectifParams::k1 value which is {0:?}"
    )]
    RectifToCamIntrisicsError(Option<f64>),

    #[error(
        "Cannot convert RectifParams to CameraIntrisicsK1Distortion struct there is no value for \
        RectifParams::k1"
    )]
    RectifToCamIntrisicsK1DistortionError,

    #[error("Error capturing camera image: {0}")]
    CameraCaptureError(std::io::Error),

    #[error("Error occured while converting an image: {0}")]
    ImageConversionError(image::ImageError),

    #[error("Error building the camera stream: {0}")]
    CamStreamBuildError(String),
    
    #[error("Error starting the camera stream: {0}")]
    CamStartError(rscam::Error),

    #[error("Provided FourCC image format code ({0}) is not supported by the image library")]
    ImageFormatError(String),

    #[error("Error recieving message from thread: {0}")]
    ChannelReceiveError(std::sync::mpsc::RecvError),

    #[error("Error while sending data through a channel")]
    ChannelSendError,

    #[error("Error while joining a thread")]
    ThreadJoinError
}
