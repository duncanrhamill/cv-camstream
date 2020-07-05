//! # Image Rectification Module
//!
//! This module provides image rectification for both mono and stereo streams.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use nalgebra::{Vector2, Point2};
use cv_pinhole::{CameraIntrinsics, CameraIntrinsicsK1Distortion};
use cv_core::{KeyPoint, CameraModel};
use serde::Deserialize;
use image::{DynamicImage, GenericImageView};

use crate::error::{Result, Error};

// -----------------------------------------------------------------------------------------------
// DATA STRUCTURES
// -----------------------------------------------------------------------------------------------

/// Recitication parameters for a single camera.
///
/// These items map directly to the [`CameraIntrinsics`] structs, with the option of including a
/// k1 parameter for radial distortion.
#[derive(Deserialize)]
pub struct RectifParams {
    /// Focal lengths (normalised by X and Y pixel sizes)
    pub focals: [f64; 2],
    
    /// Principle point in pixel coordinates
    pub principal_point: [f64; 2],

    /// Skew coefficient between the X and Y pixel sizes
    pub skew: f64,
    
    /// First distortion coefficient
    pub k1: Option<f64>
}

/// Rectification parameters for a pair of stereo cameras
#[derive(Deserialize)]
pub struct StereoRectifParams {
    /// Left hand camera parameters
    pub left: RectifParams,

    /// Right hand camera parameters
    pub right: RectifParams
}

// -----------------------------------------------------------------------------------------------
// ENUMERATIONS
// -----------------------------------------------------------------------------------------------

enum Intrisics {
    Simple(CameraIntrinsics),
    K1(CameraIntrinsicsK1Distortion)
}

// -----------------------------------------------------------------------------------------------
// IMPLEMENTATIONS
// -----------------------------------------------------------------------------------------------

impl RectifParams {
    
    /// Convert the recticication parameters into a [`CameraIntrinsics`] struct.
    ///
    /// The conversion will fail if `self.k1` is not `None`, as this would discard the value 
    /// possibly resulting in an incorrect rectification.
    pub fn to_pinhole_intrisics(&self) -> Result<CameraIntrinsics> {
        if self.k1.is_none() {
            Ok(CameraIntrinsics {
                focals: Vector2::from(self.focals),
                principal_point: Point2::from(self.principal_point),
                skew: self.skew
            })
        }
        else {
            Err(Error::RectifToCamIntrisicsError(self.k1))
        }
    }

    /// Convert the recticication parameters into a [`CameraIntrinsicsK1Distortion`] struct.
    ///
    /// The conversion will fail if `self.k1` is `None`.
    pub fn to_pinhole_intrisics_k1(&self) -> Result<CameraIntrinsicsK1Distortion> {
        if self.k1.is_some() {
            Ok(CameraIntrinsicsK1Distortion {
                simple_intrinsics: CameraIntrinsics {
                    focals: Vector2::from(self.focals),
                    principal_point: Point2::from(self.principal_point),
                    skew: self.skew
                },
                k1: self.k1.unwrap()
            })
        }
        else {
            Err(Error::RectifToCamIntrisicsK1DistortionError)
        }
    }

    /// Rectify an image using these parameters
    pub fn rectify(&self, img: &DynamicImage) -> DynamicImage {

        // New empty image of equal size and colour space to the input image
        let rect_img = new_empty_img_from_dyn_img(img);

        // Depending on whether or not there is a k1 value
        match self.k1 {
            Some(_) => {
                // If there is a k1 value use the radial distorsion coefficient as well.
                let intrinsics = self.to_pinhole_intrisics_k1().unwrap();

                // Get top left and bottom right corners of the image in normalised coordinates.
                let tl_normkp = intrinsics.calibrate(KeyPoint(Point2::from([0.0, 0.0])));
                let br_normkp = intrinsics.calibrate(KeyPoint(Point2::from(
                    [img.width() as f64, img.height() as f64]
                )));

                for y in 0..(rect_img.height() - 1) {
                    for x in 0..(rect_img.width() - 1) {
                    }
                }

                unimplemented!();

            },
            None => {
                // If no k1 value use a simple pinhole model
                let intrinsics = self.to_pinhole_intrisics().unwrap();

                let tl_normkp = intrinsics.calibrate(KeyPoint(Point2::from([0.0, 0.0])));
                let br_normkp = intrinsics.calibrate(KeyPoint(Point2::from(
                    [img.width() as f64, img.height() as f64]
                )));

                unimplemented!();
            }
        }
    }
}

// -----------------------------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// -----------------------------------------------------------------------------------------------

/// Create a new empty image from a given dynamic image
fn new_empty_img_from_dyn_img(img: &DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageBgr8(b) 
            => DynamicImage::new_bgr8(b.width(), b.height()),
        DynamicImage::ImageBgra8(b) 
            => DynamicImage::new_bgra8(b.width(), b.height()),
        DynamicImage::ImageLuma16(b) 
            => DynamicImage::new_luma16(b.width(), b.height()),
        DynamicImage::ImageLuma8(b) 
            => DynamicImage::new_luma8(b.width(), b.height()),
        DynamicImage::ImageLumaA16(b) 
            => DynamicImage::new_luma_a16(b.width(), b.height()),
        DynamicImage::ImageLumaA8(b) 
            => DynamicImage::new_luma_a8(b.width(), b.height()),
        DynamicImage::ImageRgb16(b) 
            => DynamicImage::new_rgb16(b.width(), b.height()),
        DynamicImage::ImageRgb8(b) 
            => DynamicImage::new_rgb8(b.width(), b.height()),
        DynamicImage::ImageRgba16(b) 
            => DynamicImage::new_rgba16(b.width(), b.height()),
        DynamicImage::ImageRgba8(b) 
            => DynamicImage::new_rgba8(b.width(), b.height())
    }
}
