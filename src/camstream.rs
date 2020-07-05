//! # Camera Stream Module
//!
//! This module provides a camera stream object which provides a uniform API for both mono and 
//! stereo cameras.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use image::DynamicImage;
use rscam::{Camera, Frame};

use crate::error::{Result, Error};
use crate::rectification::{RectifParams, StereoRectifParams};

// -----------------------------------------------------------------------------------------------
// TRAITS
// -----------------------------------------------------------------------------------------------

pub trait CamStream {
    type Frame;

    /// Capture a frame from the camera stream.
    fn capture(&mut self) -> Result<Self::Frame>;
}

// -----------------------------------------------------------------------------------------------
// DATA STRUCTS
// -----------------------------------------------------------------------------------------------

pub struct MonoCamStream {
    camera: Camera,

    rectif_params: Option<RectifParams>
}

pub struct StereoCamStream {
    left_cam: Camera,
    right_cam: Camera,

    rectif_params: Option<StereoRectifParams>
}

/// A frame from a stereo camera stream containing both images.
pub struct StereoFrame {
    /// The left image
    pub left: DynamicImage,

    /// The right image
    pub right: DynamicImage
}

// -----------------------------------------------------------------------------------------------
// IMPLEMENTATIONS
// -----------------------------------------------------------------------------------------------

impl CamStream for MonoCamStream {
    type Frame = DynamicImage;

    /// Capture an image from the camera.
    fn capture(&mut self) -> Result<Self::Frame> {
        // Get the frame from the camera
        let rscam_frame = self.camera.capture()
            .map_err(|e| Error::CameraCaptureError(e))?;

        // Convert the frame into an image
        let img = rscam_frame_to_dynamic_image(rscam_frame)?;

        // Rectify the images if there is a value for rectif_params
        match self.rectif_params {
            Some(ref r) => Ok(r.rectify(&img)),
            None => Ok(img)
        }
    }
}

impl CamStream for StereoCamStream {
    type Frame = StereoFrame;

    /// Capture a frame from the pair of stereo cameras.
    fn capture(&mut self) -> Result<Self::Frame> {
        // Get the frames from each camera
        let left_frame = self.left_cam.capture()
            .map_err(|e| Error::CameraCaptureError(e))?;
        let right_frame = self.right_cam.capture()
            .map_err(|e| Error::CameraCaptureError(e))?;

        // Convert the images
        let left_img = rscam_frame_to_dynamic_image(left_frame)?;
        let right_img = rscam_frame_to_dynamic_image(right_frame)?;

        // Rectify the images if there are rectif_params
        match self.rectif_params {
            Some(ref r) => {
                let left = r.left.rectify(&left_img);
                let right = r.right.rectify(&right_img);

                Ok(StereoFrame {
                    left, 
                    right
                })
            },
            None => Ok(StereoFrame {
                left: left_img,
                right: right_img
            })
        }
    }
}

// -----------------------------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// -----------------------------------------------------------------------------------------------

/// Convert an `rscam::Frame` struct into an `image::DynamicImage` struct.
fn rscam_frame_to_dynamic_image(frame: Frame) -> Result<DynamicImage> {
    image::load_from_memory(&frame)
        .map_err(|e| Error::ImageConversionError(e))
}
