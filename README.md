# Camera stream for use in the CV system

This crate provides the ability to acquire images from a camera stream, rectify those images, and return them for further processing.
Under the hood this uses [`rscam`](https://github.com/loyd/rscam) to access cameras over V4L2,
therefore currently only Linux is supported.

## Dependencies

Before installing make sure that the following dependencies are installed:

- V4L2 - video for linux 2, including the dev headers

### Ubuntu

```shell
sudo apt install v4l-utils libv4l-dev
```

## Installation

Once the dependencies are met add the following to your project's `Cargo.toml`

```toml
[dependencies]
cv_camstream = "0.1"
```

## Usage

This crate provides support for mono and stereo cameras through a builder API `CamStreamBuilder`.

```rust
// Example mono camera builder pattern
let camera = CamStreamBuilder::new()
    // Create a mono camera object
    .mono()
    // The path that the device can be found at, which returns a result
    .path("/dev/video1")
    .expect("Cannot find camera at specified path")
    // Path to the camera's rectification parameter file
    // Alternatively use .no_rectification() or .rectif_params(...)
    .rectif_params_from_file("mono_rectif_params.toml")
    .expect("Cannot find rectification parameters file")
    // Set rscam parameters, like interval, resolution, and format
    .interval(1, 30)
    .resolution(640, 480)
    .format(b"MJPG")
    .expect("Unsupported image format")
    // Construct the object
    .build()
    .expect("Failed to open camera");
```

Once the camera object has been built it is accessed through:

```rust
let img = camera.capture().expect("Failed to get camera image")
```

which returns an [`image::DynamicImage`](https://docs.rs/image/0.23.6/image/) result.

For stereo cameras the process is similar, although you must specify the left and right path
seperately through `left_path` and `right_path`. A `StereoCamStream` object returns `StereoFrame`s,
a struct which contains the left and right image respectively. Convenience functions are provided
to convert a frame into pairs (tuples) of different types of images from the `image` crate.