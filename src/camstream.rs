//! # Camera Stream Module
//!
//! This module provides a camera stream object which provides a uniform API for both mono and 
//! stereo cameras.

// -----------------------------------------------------------------------------------------------
// IMPORTS
// -----------------------------------------------------------------------------------------------

use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;

use image::{DynamicImage, GrayImage, ImageFormat};
use rscam::{Camera, Frame};

use crate::error::{Result, Error};
use crate::rectification::{RectifParams, StereoRectifParams};
use crate::GrayFloatImage;
use thread::JoinHandle;

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
    left_jh: JoinHandle<()>,
    right_jh: JoinHandle<()>,

    left_tx: Sender<WorkerCmd>,
    left_rx: Receiver<Result<(GrayFloatImage, u64)>>,

    right_tx: Sender<WorkerCmd>,
    right_rx: Receiver<Result<(GrayFloatImage, u64)>>,
}

/// A frame from a stereo camera stream containing both images.
pub struct StereoFrame {
    /// The left image
    pub left: GrayFloatImage,

    /// The right image
    pub right: GrayFloatImage,

    /// The timestamp of the left image
    pub left_timestamp: u64,

    /// The timestamp of the right image
    pub right_timestamp: u64
}

// -----------------------------------------------------------------------------------------------
// ENUMERATIONS
// -----------------------------------------------------------------------------------------------

/// Commands that can be sent by the main thread to the worker threads.
enum WorkerCmd {
    /// Capture an image from the camera
    Capture,

    /// Stop acquisition
    Stop
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

impl StereoCamStream {

    /// Create a new instance of the camera stream
    ///
    /// 
    pub(crate) fn new(
        left_cam: Camera, 
        right_cam: Camera, 
        format: ImageFormat, 
        rectif_params: Option<StereoRectifParams>
    ) -> Self {
        
        // Create all sync objects
        let (left_tx_cmd, left_rx_cmd) = channel();
        let (left_tx_img, left_rx_img) = channel();
        let (right_tx_cmd, right_rx_cmd) = channel();
        let (right_tx_img, right_rx_img) = channel();

        // Break out rectif params
        let (left_rp, right_rp) = match rectif_params {
            Some(srp) => (Some(srp.left), Some(srp.right)),
            None => (None, None)
        };

        // Start processing threads
        let left_jh = img_cap_thread(
            left_cam, 
            left_rx_cmd, 
            left_tx_img, 
            format, 
            left_rp
        );
        let right_jh = img_cap_thread(
            right_cam, 
            right_rx_cmd, 
            right_tx_img, 
            format, 
            right_rp
        );

        Self {
            left_jh,
            right_jh,
            
            left_tx: left_tx_cmd,
            left_rx: left_rx_img,

            right_tx: right_tx_cmd,
            right_rx: right_rx_img
        }
    }

    /// Stop the stream
    pub fn stop(self) -> Result<()> {
        self.left_tx.send(WorkerCmd::Stop).map_err(|_| Error::ChannelSendError)?;
        self.right_tx.send(WorkerCmd::Stop).map_err(|_| Error::ChannelSendError)?;

        self.left_jh.join().map_err(|_| Error::ThreadJoinError)?;
        self.right_jh.join().map_err(|_| Error::ThreadJoinError)?;

        Ok(())
    }
}

impl CamStream for StereoCamStream {
    type Frame = StereoFrame;

    /// Capture a frame from the pair of stereo cameras.
    fn capture(&mut self) -> Result<Self::Frame> {
        // Send the capture commands
        self.left_tx.send(WorkerCmd::Capture).map_err(|_| Error::ChannelSendError)?;
        self.right_tx.send(WorkerCmd::Capture).map_err(|_| Error::ChannelSendError)?;

        // Wait for the images from each thread
        let left = match self.left_rx.recv() {
            Ok(Ok(i)) => i,
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(Error::ChannelReceiveError(e))
        };
        let right = match self.right_rx.recv() {
            Ok(Ok(i)) => i,
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(Error::ChannelReceiveError(e))
        };

        Ok(StereoFrame {
            left: left.0,
            right: right.0,
            left_timestamp: left.1,
            right_timestamp: right.1
        })
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

/// Capture images from the given camera in a seprate thread.
fn img_cap_thread(
    cam: Camera, 
    cmd_rx: Receiver<WorkerCmd>, 
    img_tx: Sender<Result<(GrayFloatImage, u64)>>,
    format: ImageFormat,
    rectif_params: Option<RectifParams>
) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                WorkerCmd::Capture => {
                    
                    let frame = match cam.capture() {
                        Ok(f) => f,
                        Err(e) => {
                            img_tx.send(Err(Error::CameraCaptureError(e)))
                                .expect("Failed to send reply to main thread");
                            continue
                        }
                    };

                    let timestamp = frame.get_timestamp();

                    let dyn_img = match rscam_frame_to_dynamic_image(frame, format) {
                        Ok(i) => i,
                        Err(e) => {
                            img_tx.send(Err(e)).expect("Failed to send reply to main thread");
                            continue
                        }
                    };

                    let mut img = GrayFloatImage::from_dynamic(&dyn_img);

                    match rectif_params {
                        Some(r) => {
                            img = r.rectify(&img);
                        },
                        None => ()
                    };

                    img_tx.send(Ok((img, timestamp)))
                        .expect("Error sending image to main thread");
                },
                WorkerCmd::Stop => {
                    break
                }
            }
        }
    })
}