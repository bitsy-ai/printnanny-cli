#[macro_use]
extern crate clap;

use anyhow::{bail, Result};
use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use git_version::git_version;
use gstreamer::prelude::*;
use log::info;
use log::LevelFilter;

use printnanny_gst::error::ErrorMessage;
use printnanny_gst::options::{InputOption, VideoEncodingOption, VideoParameter};

pub struct BroadcastRtpVideo {
    pub host: String,
    video_port: i32,
}

pub struct BroadcastRtpVideoOverlay {
    pub host: String,
    pub video_port: i32,
    pub data_port: i32,
    pub overlay_port: i32,
}

pub enum AppVariant {
    // broadcast source video stream over 1 rtp port (light compute)
    BroadcastRtpVideo(BroadcastRtpVideo),
    // broadcast source video, model inference video, and model inference tensor over 3 rtp ports (medium compute)
    BroadcastRtpTfliteOverlay(BroadcastRtpVideoOverlay),
    // broadcast video composited from source / inference (heavy compute)
    BroadcastRtpTfliteComposite(BroadcastRtpVideoOverlay),
}

pub struct App<'a> {
    video: VideoParameter,
    input: InputOption,
    height: i32,
    width: i32,
    required_plugins: Vec<&'a str>,
    variant: AppVariant,
}

impl App<'_> {
    pub fn new(args: &ArgMatches, sub_args: &ArgMatches, subcommand: &str) -> Result<Self> {
        let mut required_plugins = vec!["videoconvert", "videoscale"];
        // input src requirement
        let input = args.value_of_t("input")?;
        let mut input_reqs = match input {
            InputOption::Libcamerasrc => vec!["libcamerasrc"],
            InputOption::Videotestsrc => vec!["videotestsrc"],
        };
        required_plugins.append(&mut input_reqs);
        // encode in software vs hardware-accelerated
        let encoder_opt: VideoEncodingOption = args.value_of_t("encoder")?;
        let video: VideoParameter = encoder_opt.into();
        let mut encoder_reqs = video.requirements.split(' ').collect::<Vec<&str>>();
        required_plugins.append(&mut encoder_reqs);

        // tensorflow and nnstreamer requirements
        let variant: AppVariant = match subcommand {
            "broadcast-rtp-video" => {
                // append rtp broadcast requirements
                let mut reqs = vec!["rtp", "udp"];
                required_plugins.append(&mut reqs);
                let host = sub_args.value_of("host").unwrap().into();
                let video_port: i32 = sub_args.value_of_t("video_port").unwrap();
                let subapp = BroadcastRtpVideo { host, video_port };
                AppVariant::BroadcastRtpVideo(subapp)
            }
            "broadcast-rtp-tflite" => {
                // append rtp broadcast and tflite requirements
                let mut reqs = vec![
                    "tensor_converter",
                    "tensor_transform",
                    "tensor_filter",
                    "tensor_decoder",
                    "rtp",
                    "udp",
                ];
                required_plugins.append(&mut reqs);
                let host = sub_args.value_of("host").unwrap().into();
                let video_port: i32 = sub_args.value_of_t("video_port").unwrap();
                let data_port: i32 = sub_args.value_of_t("data_port").unwrap();
                let overlay_port: i32 = sub_args.value_of_t("overlay_port").unwrap();
                let subapp = BroadcastRtpVideoOverlay {
                    host,
                    video_port,
                    data_port,
                    overlay_port,
                };
                AppVariant::BroadcastRtpTfliteOverlay(subapp)
            }
            _ => bail!("Received unknown subcommand {}", subcommand),
        };

        let height: i32 = args.value_of_t("height").unwrap_or(480);
        let width: i32 = args.value_of_t("width").unwrap_or(480);

        Ok(Self {
            video,
            input,
            required_plugins,
            height,
            width,
            variant,
        })
    }

    pub fn check_plugins(&self) -> Result<()> {
        let registry = gstreamer::Registry::get();
        let missing = self
            .required_plugins
            .iter()
            .filter(|n| registry.find_plugin(n).is_none())
            .cloned()
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            bail!("Missing plugins: {:?}", missing);
        } else {
            Ok(())
        }
    }
    // build a video-only pipeline without tflite inference
    fn build_broadcast_rtp_video_pipeline(
        &self,
        app: &BroadcastRtpVideo,
    ) -> Result<gstreamer::Pipeline> {
        let p = format!(
            "{}
            ! capsfilter caps=video/x-raw,width={},height={},framerate=0/1
            ! {} 
            ! {}
            ! udpsink host={} port={} ",
            &self.input,
            &self.width,
            &self.height,
            &self.video.encoder,
            &self.video.payloader,
            &app.host,
            &app.video_port
        );
        let pipeline = gstreamer::parse_launch(&p)?;
        Ok(pipeline
            .downcast::<gstreamer::Pipeline>()
            .expect("Invalid gstreamer pipeline"))
    }

    // build a tflite pipeline where inference results are rendered to overlay
    fn build_broadcast_rtp_tflite_overlay_pipeline(
        &self,
        app: &BroadcastRtpVideoOverlay,
    ) -> Result<gstreamer::Pipeline> {
        unimplemented!("build_broadcast_rtp_tflite_overlay_pipeline is not yet implemented")
    }

    // build a tflite pipeline where inference results are composited to overlay
    fn build_broadcast_rtp_tflite_composite_pipeline(
        &self,
        app: &BroadcastRtpVideoOverlay,
    ) -> Result<gstreamer::Pipeline> {
        unimplemented!("build_broadcast_rtp_tflite_composite_pipeline is not yet implemented")
    }

    fn build_tflite_pipeline(&self) -> Result<gstreamer::Pipeline> {
        let p = format!(
            "{} \
            ! capsfilter caps=video/x-raw,format=RGB,width={},height={},framerate=0/1
            ! {} 
            ! {}
            ! testsink ",
            &self.input, &self.width, &self.height, &self.video.encoder, &self.video.payloader
        );
        let pipeline = gstreamer::parse_launch(&p)?;
        Ok(pipeline
            .downcast::<gstreamer::Pipeline>()
            .expect("Invalid gstreamer pipeline"))
    }

    pub fn build_pipeline(&self) -> Result<gstreamer::Pipeline> {
        match &self.variant {
            AppVariant::BroadcastRtpVideo(app) => self.build_broadcast_rtp_video_pipeline(app),
            AppVariant::BroadcastRtpTfliteOverlay(app) => {
                self.build_broadcast_rtp_tflite_overlay_pipeline(app)
            }
            AppVariant::BroadcastRtpTfliteComposite(app) => {
                self.build_broadcast_rtp_tflite_composite_pipeline(app)
            }
        }
    }

    pub fn play(&self) -> Result<()> {
        let pipeline = self.build_pipeline()?;
        info!("Setting pipeline {:?} state to Playing", pipeline);
        pipeline.set_state(gstreamer::State::Playing)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    // include git sha in version, which requires passing a boxed string to clap's .version() builder
    let version = Box::leak(format!("{} {}", crate_version!(), git_version!()).into_boxed_str());

    // parse args
    let app_name = "printnanny-gst";

    let app = Command::new(app_name)
        .author(crate_authors!())
        .about(crate_description!())
        .version(&version[..])
        .subcommand_required(true)
        // generic app args
        .arg(
            Arg::new("v")
                .short('v')
                .multiple_occurrences(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::new("height")
                .long("height")
                .default_value("480")
                .takes_value(true)
                .help("Input resolution height"),
        )
        .arg(
            Arg::new("width")
                .long("width")
                .default_value("640")
                .takes_value(true)
                .help("Input resolution width"),
        )
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .required(true)
                .takes_value(true)
                .possible_values(InputOption::possible_values())
                .help(""),
        )
        .arg(
            Arg::new("encoder")
                .short('e')
                .long("encoder")
                .required(true)
                .takes_value(true)
                .possible_values(VideoEncodingOption::possible_values())
                .help("Run TensorFlow lite model on output"),
        )
        // tflite app args
        .subcommand(
            Command::new("broadcast-rtp-tflite")
                .author(crate_authors!())
                .about(
                "Run TensorFlow Lite inference over stream, broadcast encoded video stream and inference results over rtp",
            ),
        )
        // simple video app args
        .subcommand(
            Command::new("broadcast-rtp-video")
                .author(crate_authors!())
                .about("Encode video and broadcast over rtp")
            .arg(
                Arg::new("host")
                    .long("host")
                    .default_value("localhost")
                    .takes_value(true)
                    .help("udpsink host value"),
            )
            .arg(
                Arg::new("video_port")
                    .long("video-port")
                    .default_value("5104")
                    .takes_value(true)
                    .help("udpsink port value"),
            )
        );

    let app_m = app.get_matches();
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'printnanny v v v' or 'printnanny vvv' vs 'printnanny v'
    let verbosity = app_m.occurrences_of("v");
    let mut builder = Builder::new();
    match verbosity {
        0 => builder.filter_level(LevelFilter::Warn).init(),
        1 => builder.filter_level(LevelFilter::Info).init(),
        2 => builder.filter_level(LevelFilter::Debug).init(),
        _ => builder.filter_level(LevelFilter::Trace).init(),
    };

    // Initialize GStreamer first
    gstreamer::init()?;
    // Check required_plugins plugins are installed

    let (subcommand, sub_m) = app_m.subcommand().unwrap();
    let app = App::new(&app_m, &sub_m, &subcommand)?;

    app.check_plugins()?;
    app.play()?;

    Ok(())
}