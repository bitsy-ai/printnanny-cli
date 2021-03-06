use clap::{ArgEnum, PossibleValue};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum AppModeOption {
    // broadcast source video stream over 1 rtp port (light compute)
    RtpVideo,
    // broadcast source video, model inference video, and model inference tensor over 3 rtp ports (medium compute)
    RtpTfliteOverlay,
    // broadcast video composited from source / inference (heavy compute)
    RtpTfliteComposite,
}

impl AppModeOption {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        AppModeOption::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for AppModeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for AppModeOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum SinkOption {
    Fakesink,
    Udpsink,
}

impl SinkOption {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        SinkOption::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for SinkOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for SinkOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum SrcOption {
    Libcamerasrc,
    Videotestsrc,
}

impl SrcOption {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        SrcOption::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for SrcOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for SrcOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

#[derive(Debug)]
pub struct VideoParameter {
    pub encoder: &'static str,
    pub encoding_name: &'static str,
    pub payloader: &'static str,
    pub parser: &'static str,
    pub requirements: &'static str,
}

pub const H264_SOFTWARE: VideoParameter = VideoParameter {
    requirements: "x264",
    encoder: "x264enc tune=zerolatency",
    encoding_name: "h264",
    parser: "h264parse",
    payloader: "rtph264pay aggregate-mode=zero-latency",
};

pub const H264_HARDWARE: VideoParameter = VideoParameter {
    requirements: "video4linux2",
    encoder: "v4l2h264enc extra-controls=controls,repeat_sequence_header=1",
    encoding_name: "h264",
    parser: "h264parse",
    payloader: "rtph264pay aggregate-mode=zero-latency",
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum VideoEncodingOption {
    H264Software,
    H264Hardware,
}

impl From<VideoEncodingOption> for VideoParameter {
    fn from(opt: VideoEncodingOption) -> Self {
        match opt {
            VideoEncodingOption::H264Hardware => H264_HARDWARE,
            VideoEncodingOption::H264Software => H264_SOFTWARE,
        }
    }
}

impl VideoEncodingOption {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        VideoEncodingOption::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

impl std::fmt::Display for VideoEncodingOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for VideoEncodingOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}
