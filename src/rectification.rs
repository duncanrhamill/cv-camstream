//! # Image Rectification Module
//!
//! This module provides image rectification for both mono and stereo streams.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use nalgebra::{Vector2, Point2};
use cv_pinhole::{CameraIntrinsics, CameraIntrinsicsK1Distortion, NormalizedKeyPoint};
use cv_core::{KeyPoint, CameraModel};
use serde::Deserialize;
use image::{DynamicImage, GenericImageView};

use crate::error::{Result, Error};
use crate::GrayFloatImage;

// -----------------------------------------------------------------------------------------------
// DATA STRUCTURES
// -----------------------------------------------------------------------------------------------

/// Recitication parameters for a single camera.
///
/// These items map directly to the [`CameraIntrinsics`] structs, with the option of including a
/// k1 parameter for radial distortion.
#[derive(Deserialize, Debug)]
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
#[derive(Deserialize, Debug)]
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
    pub fn rectify(&self, img: &DynamicImage) -> GrayFloatImage {

        // Get a gray float image from the dynamic image
        let grey_img = GrayFloatImage::from_dynamic(img);

        // New empty image of equal size and colour space to the input image
        let mut rect_img = GrayFloatImage::new(
            img.width() as usize, 
            img.height() as usize
        );

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

                for y in 0..rect_img.height() as u32 {
                    for x in 0..rect_img.width() as u32 {
                        // Get the normalised keypoint value for this position
                        let normkp = image_xy_to_normkp(
                            x, y,
                            rect_img.width() as u32, rect_img.height() as u32,
                            tl_normkp, br_normkp
                        );

                        // Reproject to find the keypoint coordinates
                        let kp = intrinsics.uncalibrate(normkp);

                        // Set the pixel value for the new image
                        *rect_img.0.get_pixel_mut(x, y) = linterp_pixels(kp, &grey_img);
                    }   
                }

                rect_img

            },
            None => {
                // If no k1 value use a simple pinhole model
                let intrinsics = self.to_pinhole_intrisics().unwrap();

                let tl_normkp = intrinsics.calibrate(KeyPoint(Point2::from([0.0, 0.0])));
                let br_normkp = intrinsics.calibrate(KeyPoint(Point2::from(
                    [img.width() as f64, img.height() as f64]
                )));

                for y in 0..rect_img.height() as u32 {
                    for x in 0..rect_img.width() as u32 {
                        // Get the normalised keypoint value for this position
                        let normkp = image_xy_to_normkp(
                            x, y,
                            rect_img.width() as u32, rect_img.height() as u32,
                            tl_normkp, br_normkp
                        );

                        // Reproject to find the keypoint coordinates
                        let kp = intrinsics.uncalibrate(normkp);

                        // Set the pixel value for the new image
                        *rect_img.0.get_pixel_mut(x, y) = linterp_pixels(kp, &grey_img);
                    }   
                }

                rect_img
            }
        }
    }
}

// -----------------------------------------------------------------------------------------------
// PRIVATE FUNCTIONS
// -----------------------------------------------------------------------------------------------

/// Converts an (x, y) integer pixel coordinate into a normalised keypoint coordinate.
///
/// This function conceptually places the integer coordinates at the centre of the pixel, not the
/// top left.
#[inline]
fn image_xy_to_normkp(
    x: u32, y: u32,
    width: u32, height: u32,
    tl_normkp: NormalizedKeyPoint,
    br_normkp: NormalizedKeyPoint
) -> NormalizedKeyPoint {
    
    // Get width and height in normalised coordinates
    let nw = tl_normkp.0.x - br_normkp.0.x;
    let nh = tl_normkp.0.y - br_normkp.0.y;

    // Calculate pixel size in normalised coordinates
    let nhx = nw / width as f64;
    let nhy = nh / height as f64;

    // Finally the normalised point is the top_left - how ever many normalised pixels away we are.
    // This is negative because the sense is reversed. 0.5 is added to move the coordinate to the
    // centre of the pixel, not the top-left.
    NormalizedKeyPoint(Point2::from([
        tl_normkp.0.x - nhx*(x as f64 + 0.5),
        tl_normkp.0.y - nhy*(y as f64 + 0.5)
    ]))
}

#[inline]
fn linterp_pixels(kp: KeyPoint, img: &GrayFloatImage) -> image::Luma<f32>
{
    // If the keypoint is negative return black
    if kp.0.x < 0.0 || kp.0.y < 0.0 {
        return image::Luma::from([0.0])
    }

    // Get the keypoint in u32 coordas
    let x = kp.0.x.floor() as usize;
    let y = kp.0.y.floor() as usize;

    // If the keypoints are outside the image return black
    if x >= img.width() - 1 || y >= img.height() - 1 {
        return image::Luma::from([0.0])
    }

    // Get the fractional parts of the coordinates, which will affect how much light from 
    // neigbouring pixels is used.
    let fract_x = kp.0.x.fract() as f32;
    let fract_y = kp.0.y.fract() as f32;

    // Interpolate the pixel value
    let brightness = 0.5 * (
        img.get(x, y) * (2.0 - fract_x - fract_y)
        + (img.get(x + 1, y) * fract_x)
        + (img.get(x, y + 1) * fract_y)
    );
    
    image::Luma([brightness])
}