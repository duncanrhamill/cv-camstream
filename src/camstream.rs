//! # Camera Stream Module
//!
//! This module provides a camera stream object which provides a uniform API for both mono and 
//! stereo cameras.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use image::{DynamicImage, GrayImage, ImageFormat};
use rscam::{Camera, Frame};

use crate::error::{Result, Error};
use crate::rectification::{RectifParams, StereoRectifParams};
use crate::GrayFloatImage;

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

    img_format: ImageFormat,

    rectif_params: Option<RectifParams>
}

pub struct StereoCamStream {
    pub(crate) left_cam: Camera,
    pub(crate) right_cam: Camera,

    pub(crate) img_format: ImageFormat,

    pub(crate) rectif_params: Option<StereoRectifParams>
}

/// A frame from a stereo camera stream containing both images.
pub struct StereoFrame {
    /// The left image
    pub left: GrayFloatImage,

    /// The right image
    pub right: GrayFloatImage
}

// -----------------------------------------------------------------------------------------------
// IMPLEMENTATIONS
// -----------------------------------------------------------------------------------------------

impl CamStream for MonoCamStream {
    type Frame = GrayFloatImage;

    /// Capture an image from the camera.
    fn capture(&mut self) -> Result<Self::Frame> {
        // Get the frame from the camera
        let rscam_frame = self.camera.capture()
            .map_err(|e| Error::CameraCaptureError(e))?;

        // Convert the frame into an image
        let img = GrayFloatImage::from_dynamic(
            &rscam_frame_to_dynamic_image(rscam_frame, self.img_format)?
        );

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
        let left_img = GrayFloatImage::from_dynamic(
            &rscam_frame_to_dynamic_image(left_frame, self.img_format)?
        );
        let right_img = GrayFloatImage::from_dynamic(
            &rscam_frame_to_dynamic_image(right_frame, self.img_format)?
        );

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

impl StereoFrame {

    /// Get the width of an individual image in the frame
    pub fn width(&self) -> u32 {
        self.left.width() as u32
    }

    /// Get the height of an individual image in the frame
    pub fn height(&self) -> u32 {
        self.left.height() as u32
    }

    /// Convert the frame into a pair of luma images
    pub fn to_luma8_pair(self) -> (GrayImage, GrayImage) {
        (self.left.to_dynamic_luma8().to_luma(), self.right.to_dynamic_luma8().to_luma())
    }
}

// -----------------------------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// -----------------------------------------------------------------------------------------------

/// Convert an `rscam::Frame` struct into an `image::DynamicImage` struct.
fn rscam_frame_to_dynamic_image(frame: Frame, format: ImageFormat) -> Result<DynamicImage> {
    image::load_from_memory_with_format(&frame, format)
        .map_err(|e| Error::ImageConversionError(e))
}