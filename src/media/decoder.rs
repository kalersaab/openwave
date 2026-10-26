use crate::error::{OpenWaveError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Maximum number of bytes read while probing a file. Container metadata
/// (including Matroska tracks and Ogg identification headers) normally lives
/// well inside this window, keeping format detection comfortably below the
/// 500 ms budget.
pub const DEFAULT_MAX_PROBE_BYTES: usize = 2 * 1024 * 1024;

const MAX_MOOV_BYTES: u64 = 64 * 1024 * 1024;
const EBML_MAGIC: [u8; 4] = [0x1A, 0x45, 0xDF, 0xA3];
const ASF_HEADER_GUID: [u8; 16] = [
    0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
];
const ASF_VIDEO_MEDIA_GUID: [u8; 16] = [
    0xC0, 0xEF, 0x19, 0xBC, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContainerFormat {
    Mp4,
    Mkv,
    Avi,
    Mov,
    Flv,
    WebM,
    Ogg,
    Mpeg,
    Asf,
}

impl ContainerFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "MP4",
            Self::Mkv => "MKV",
            Self::Avi => "AVI",
            Self::Mov => "MOV",
            Self::Flv => "FLV",
            Self::WebM => "WebM",
            Self::Ogg => "OGG",
            Self::Mpeg => "MPEG",
            Self::Asf => "ASF",
        }
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        match normalize_extension_str(extension) {
            "mp4" | "m4v" | "m4p" => Some(Self::Mp4),
            "mkv" | "mka" => Some(Self::Mkv),
            "avi" => Some(Self::Avi),
            "mov" => Some(Self::Mov),
            "flv" => Some(Self::Flv),
            "webm" => Some(Self::WebM),
            "ogg" | "oga" | "ogv" => Some(Self::Ogg),
            "mpg" | "mpeg" | "m2v" | "ts" | "m2ts" | "mts" | "vob" => Some(Self::Mpeg),
            "asf" | "wma" | "wmv" => Some(Self::Asf),
            _ => None,
        }
    }
}

impl std::fmt::Display for ContainerFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFormat {
    Mp3,
    Flac,
    Wav,
    Aac,
    Ogg,
    Wma,
    Opus,
}

impl AudioFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "MP3",
            Self::Flac => "FLAC",
            Self::Wav => "WAV",
            Self::Aac => "AAC",
            Self::Ogg => "OGG",
            Self::Wma => "WMA",
            Self::Opus => "OPUS",
        }
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        match normalize_extension_str(extension) {
            "mp3" => Some(Self::Mp3),
            "flac" => Some(Self::Flac),
            "wav" | "wave" => Some(Self::Wav),
            "aac" | "m4a" | "m4b" | "m4p" => Some(Self::Aac),
            "ogg" | "oga" => Some(Self::Ogg),
            "wma" => Some(Self::Wma),
            "opus" => Some(Self::Opus),
            _ => None,
        }
    }
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VideoCodec {
    H264,
    H265,
    Vp8,
    Vp9,
    Av1,
    Mpeg2,
    Mpeg4,
    Mjpeg,
    Vp6,
    H263,
    Theora,
    Other(String),
}

impl VideoCodec {
    pub fn as_str(&self) -> &str {
        match self {
            Self::H264 => "H.264",
            Self::H265 => "H.265",
            Self::Vp8 => "VP8",
            Self::Vp9 => "VP9",
            Self::Av1 => "AV1",
            Self::Mpeg2 => "MPEG-2",
            Self::Mpeg4 => "MPEG-4",
            Self::Mjpeg => "MJPEG",
            Self::Vp6 => "VP6",
            Self::H263 => "H.263",
            Self::Theora => "Theora",
            Self::Other(name) => name,
        }
    }
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioCodec {
    Aac,
    Mp3,
    Flac,
    Vorbis,
    Opus,
    Pcm,
    Ac3,
    Eac3,
    Wma,
    Alac,
    Speex,
    Mp2,
    Mp1,
    Other(String),
}

impl AudioCodec {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Aac => "AAC",
            Self::Mp3 => "MP3",
            Self::Flac => "FLAC",
            Self::Vorbis => "Vorbis",
            Self::Opus => "Opus",
            Self::Pcm => "PCM",
            Self::Ac3 => "AC-3",
            Self::Eac3 => "E-AC-3",
            Self::Wma => "WMA",
            Self::Alac => "ALAC",
            Self::Speex => "Speex",
            Self::Mp2 => "MP2",
            Self::Mp1 => "MP1",
            Self::Other(name) => name,
        }
    }
}

impl std::fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Audio,
    Video,
    AudioVideo,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Codec {
    Video(VideoCodec),
    Audio(AudioCodec),
}

impl std::fmt::Display for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Video(codec) => write!(f, "video/{codec}"),
            Self::Audio(codec) => write!(f, "audio/{codec}"),
        }
    }
}

/// Result of probing a media file's container and stream codecs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaFormat {
    pub container: Option<ContainerFormat>,
    /// Set when the file is an audio-only media format such as MP3, FLAC or
    /// an Opus-in-Ogg stream.
    pub audio_format: Option<AudioFormat>,
    pub kind: MediaKind,
    pub video_codecs: Vec<VideoCodec>,
    pub audio_codecs: Vec<AudioCodec>,
}

impl MediaFormat {
    fn build(
        container: Option<ContainerFormat>,
        audio_format: Option<AudioFormat>,
        mut video_codecs: Vec<VideoCodec>,
        mut audio_codecs: Vec<AudioCodec>,
        extension: Option<&str>,
    ) -> Self {
        dedup(&mut video_codecs);
        dedup(&mut audio_codecs);

        let audio_format = if video_codecs.is_empty() {
            audio_format
                .or_else(|| infer_audio_format(&audio_codecs))
                .or_else(|| {
                    if audio_codecs.is_empty() && container.is_none() {
                        extension.and_then(AudioFormat::from_extension)
                    } else {
                        None
                    }
                })
        } else {
            None
        };

        let kind = if !video_codecs.is_empty() {
            if audio_codecs.is_empty() {
                MediaKind::Video
            } else {
                MediaKind::AudioVideo
            }
        } else if !audio_codecs.is_empty() || audio_format.is_some() {
            MediaKind::Audio
        } else {
            kind_from_extension(extension)
        };

        Self {
            container,
            audio_format,
            kind,
            video_codecs,
            audio_codecs,
        }
    }

    pub fn is_audio(&self) -> bool {
        self.kind == MediaKind::Audio
    }

    pub fn is_video(&self) -> bool {
        matches!(self.kind, MediaKind::Video | MediaKind::AudioVideo)
    }

    pub fn required_codecs(&self) -> Vec<Codec> {
        self.video_codecs
            .iter()
            .cloned()
            .map(Codec::Video)
            .chain(self.audio_codecs.iter().cloned().map(Codec::Audio))
            .collect()
    }
}

/// Detects media containers and the codecs required to decode them.
#[derive(Debug, Clone)]
pub struct FormatDetector {
    max_probe_bytes: usize,
}

impl Default for FormatDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatDetector {
    pub fn new() -> Self {
        Self {
            max_probe_bytes: DEFAULT_MAX_PROBE_BYTES,
        }
    }

    /// Limit how much of the file header is read during detection.
    pub fn with_max_probe_bytes(mut self, max_probe_bytes: usize) -> Self {
        self.max_probe_bytes = max_probe_bytes.max(16);
        self
    }

    pub fn detect(&self, path: impl AsRef<Path>) -> Result<MediaFormat> {
        let path = path.as_ref();
        let extension = extension_of(path);

        let mut file = File::open(path).map_err(|error| OpenWaveError::io(path, &error))?;
        let file_len = file
            .metadata()
            .map_err(|error| OpenWaveError::io(path, &error))?
            .len();

        let probe_len = file_len.min(self.max_probe_bytes as u64) as usize;
        let mut probe = Vec::with_capacity(probe_len);
        (&mut file)
            .take(probe_len as u64)
            .read_to_end(&mut probe)
            .map_err(|error| OpenWaveError::io(path, &error))?;

        let mut format = self
            .detect_bytes(&probe, extension.as_deref())
            .map_err(|error| error.with_path(path))?;

        // An MP4/MOV file can keep its `moov` atom (which holds the codec
        // descriptions) after a large `mdat`. The initial probe may not reach
        // it, so walk the top-level atoms directly instead of reading the
        // whole file.
        if matches!(
            format.container,
            Some(ContainerFormat::Mp4 | ContainerFormat::Mov)
        ) && format.video_codecs.is_empty()
            && format.audio_codecs.is_empty()
        {
            let (video, audio) = scan_iso_file(&mut file, file_len)
                .map_err(|error| OpenWaveError::io(path, &error))?;

            if !video.is_empty() || !audio.is_empty() {
                format =
                    MediaFormat::build(format.container, None, video, audio, extension.as_deref());
            }
        }

        Ok(format)
    }

    /// Detect a format from an in-memory header or an entire small file.
    /// `extension` is only used as a hint for ambiguous containers and for
    /// audio-only extension fallbacks.
    pub fn detect_bytes(&self, data: &[u8], extension: Option<&str>) -> Result<MediaFormat> {
        let normalized = extension.map(|value| normalize_extension_str(value).to_ascii_lowercase());
        let ext = normalized.as_deref();

        if data.is_empty() {
            return Err(OpenWaveError::unsupported_format("empty file"));
        }

        if data.starts_with(b"RIFF") {
            if data.len() < 12 {
                return Err(OpenWaveError::unsupported_format("truncated RIFF header"));
            }
            return match &data[8..12] {
                b"AVI " => Ok(detect_avi(data, ext)),
                b"WAVE" => Ok(detect_wav(data, ext)),
                other => Err(OpenWaveError::unsupported_format(format!(
                    "unsupported RIFF subtype '{}'",
                    String::from_utf8_lossy(other)
                ))),
            };
        }

        if data.len() >= ASF_HEADER_GUID.len() && data[..16] == ASF_HEADER_GUID {
            return Ok(detect_asf(data, ext));
        }

        if data.starts_with(b"OggS") {
            return Ok(detect_ogg(data, ext));
        }

        if data.starts_with(&EBML_MAGIC) {
            return Ok(detect_ebml(data, ext));
        }

        if data.starts_with(b"FLV") {
            return Ok(detect_flv(data, ext));
        }

        if data.starts_with(b"fLaC") {
            return Ok(MediaFormat::build(
                None,
                Some(AudioFormat::Flac),
                Vec::new(),
                vec![AudioCodec::Flac],
                ext,
            ));
        }

        if data.starts_with(b"ID3") {
            return Ok(detect_id3(data, ext));
        }

        if is_adts(data) {
            return Ok(MediaFormat::build(
                None,
                Some(AudioFormat::Aac),
                Vec::new(),
                vec![AudioCodec::Aac],
                ext,
            ));
        }

        if let Some(codec) = mpeg_audio_codec_at(data, 0) {
            let audio_format = match codec {
                AudioCodec::Mp3 => Some(AudioFormat::Mp3),
                _ => None,
            };
            return Ok(MediaFormat::build(
                None,
                audio_format,
                Vec::new(),
                vec![codec],
                ext,
            ));
        }

        if is_iso_bmff(data) {
            return detect_iso(data, ext);
        }

        if is_mpeg_program_stream(data) || is_mpeg_transport_stream(data, ext) {
            return Ok(detect_mpeg(data, ext));
        }

        Err(OpenWaveError::unsupported_format(
            "unrecognised file signature",
        ))
    }
}

pub fn detect_format(path: impl AsRef<Path>) -> Result<MediaFormat> {
    FormatDetector::new().detect(path)
}

fn normalize_extension_str(extension: &str) -> &str {
    extension.trim_start_matches('.').trim()
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| normalize_extension_str(extension).to_ascii_lowercase())
}

fn infer_audio_format(audio_codecs: &[AudioCodec]) -> Option<AudioFormat> {
    if audio_codecs.contains(&AudioCodec::Opus) {
        Some(AudioFormat::Opus)
    } else if audio_codecs.contains(&AudioCodec::Flac) {
        Some(AudioFormat::Flac)
    } else if audio_codecs.contains(&AudioCodec::Aac) {
        Some(AudioFormat::Aac)
    } else if audio_codecs.contains(&AudioCodec::Wma) {
        Some(AudioFormat::Wma)
    } else if audio_codecs.contains(&AudioCodec::Mp3) {
        Some(AudioFormat::Mp3)
    } else if audio_codecs.contains(&AudioCodec::Vorbis) {
        Some(AudioFormat::Ogg)
    } else {
        None
    }
}

fn kind_from_extension(extension: Option<&str>) -> MediaKind {
    match extension {
        Some(
            "mp3" | "flac" | "wav" | "wave" | "aac" | "m4a" | "m4b" | "m4p" | "wma" | "opus"
            | "oga" | "mka",
        ) => MediaKind::Audio,
        Some(
            "mp4" | "m4v" | "mkv" | "avi" | "mov" | "flv" | "webm" | "mpg" | "mpeg" | "m2v" | "ts"
            | "m2ts" | "mts" | "vob" | "ogv" | "wmv",
        ) => MediaKind::Video,
        _ => MediaKind::Unknown,
    }
}

fn dedup<T: PartialEq>(items: &mut Vec<T>) {
    let mut index = 0;
    while index < items.len() {
        let duplicate = items[..index].contains(&items[index]);
        if duplicate {
            items.remove(index);
        } else {
            index += 1;
        }
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    find_subslice(haystack, needle).is_some()
}

fn be24(bytes: &[u8]) -> usize {
    ((bytes[0] as usize) << 16) | ((bytes[1] as usize) << 8) | bytes[2] as usize
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn be64(bytes: &[u8]) -> u64 {
    u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn is_adts(data: &[u8]) -> bool {
    data.len() >= 7 && data[0] == 0xFF && (data[1] & 0xF6) == 0xF0
}

fn mpeg_audio_codec_at(data: &[u8], offset: usize) -> Option<AudioCodec> {
    let header = data.get(offset..offset + 4)?;
    if header[0] != 0xFF || (header[1] & 0xE0) != 0xE0 {
        return None;
    }

    let layer = (header[1] >> 1) & 0x03;
    let bitrate_index = (header[2] >> 4) & 0x0F;
    let sample_rate_index = (header[2] >> 2) & 0x03;
    if layer == 0 || bitrate_index == 0x0F || sample_rate_index == 0x03 {
        return None;
    }

    Some(match layer {
        1 => AudioCodec::Mp3,
        2 => AudioCodec::Mp2,
        3 => AudioCodec::Mp1,
        _ => return None,
    })
}

fn detect_id3(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let payload = id3_payload(data);

    let (audio_format, codec) = if is_adts(payload) {
        (Some(AudioFormat::Aac), AudioCodec::Aac)
    } else if let Some(codec) = mpeg_audio_codec_at(payload, 0) {
        let format = match codec {
            AudioCodec::Mp3 => Some(AudioFormat::Mp3),
            _ => None,
        };
        (format, codec)
    } else {
        (Some(AudioFormat::Mp3), AudioCodec::Mp3)
    };

    MediaFormat::build(None, audio_format, Vec::new(), vec![codec], extension)
}

fn id3_payload(data: &[u8]) -> &[u8] {
    if data.len() < 10 {
        return &[];
    }

    let size = ((data[6] as usize & 0x7F) << 21)
        | ((data[7] as usize & 0x7F) << 14)
        | ((data[8] as usize & 0x7F) << 7)
        | (data[9] as usize & 0x7F);

    data.get(10 + size..).unwrap_or(&[])
}

fn detect_wav(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let codec = wave_format_tag(data)
        .map(codec_from_wave_tag)
        .unwrap_or(AudioCodec::Pcm);

    MediaFormat::build(
        None,
        Some(AudioFormat::Wav),
        Vec::new(),
        vec![codec],
        extension,
    )
}

fn wave_format_tag(data: &[u8]) -> Option<u16> {
    let mut position = 12usize;
    while position + 8 <= data.len() {
        let id = &data[position..position + 4];
        let size = le32(&data[position + 4..position + 8]) as usize;
        let body = position + 8;

        if id == b"fmt " && body + 2 <= data.len() {
            let mut tag = le16(&data[body..body + 2]);
            if tag == 0xFFFE && body + 26 <= data.len() {
                tag = le16(&data[body + 24..body + 26]);
            }
            return Some(tag);
        }

        let next = body.checked_add(size)?;
        if next <= position {
            break;
        }
        position = next + (size & 1);
    }

    None
}

fn codec_from_wave_tag(tag: u16) -> AudioCodec {
    match tag {
        0x0001 | 0x0003 | 0x0006 | 0x0007 => AudioCodec::Pcm,
        0x0050 => AudioCodec::Mp2,
        0x0055 => AudioCodec::Mp3,
        0x00FF => AudioCodec::Aac,
        0x0161..=0x0163 => AudioCodec::Wma,
        0x2000 => AudioCodec::Ac3,
        0x704F => AudioCodec::Opus,
        0xF1AC => AudioCodec::Flac,
        0x0002 => AudioCodec::Other("ADPCM".to_string()),
        _ => AudioCodec::Other(format!("WAV format 0x{tag:04X}")),
    }
}

fn detect_avi(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();

    let mut search_from = 0usize;
    while search_from + 4 <= data.len() {
        let Some(relative) = find_subslice(&data[search_from..], b"strh") else {
            break;
        };
        let strh_position = search_from + relative;
        let payload = strh_position + 8;
        if payload + 8 > data.len() {
            break;
        }

        let stream_kind = &data[payload..payload + 4];
        let handler = &data[payload + 4..payload + 8];
        let region_end = find_subslice(&data[payload..], b"strh")
            .map(|offset| payload + offset)
            .unwrap_or(data.len());
        let region = &data[payload..region_end];

        if stream_kind == b"vids" {
            if let Some(codec) = avi_video_handler(handler) {
                video_codecs.push(codec);
            } else if let Some(compression) = avi_video_compression(region)
                && let Some(codec) = avi_video_handler(compression)
            {
                video_codecs.push(codec);
            }
        } else if stream_kind == b"auds" {
            if let Some(codec) = avi_audio_handler(handler) {
                audio_codecs.push(codec);
            } else if let Some(codec) = avi_audio_format(region) {
                audio_codecs.push(codec);
            }
        }

        search_from = strh_position + 4;
    }

    MediaFormat::build(
        Some(ContainerFormat::Avi),
        None,
        video_codecs,
        audio_codecs,
        extension,
    )
}

fn avi_video_handler(handler: &[u8]) -> Option<VideoCodec> {
    if handler.eq_ignore_ascii_case(b"H264")
        || handler.eq_ignore_ascii_case(b"AVC1")
        || handler.eq_ignore_ascii_case(b"X264")
    {
        Some(VideoCodec::H264)
    } else if handler.eq_ignore_ascii_case(b"HEVC")
        || handler.eq_ignore_ascii_case(b"H265")
        || handler.eq_ignore_ascii_case(b"HVC1")
    {
        Some(VideoCodec::H265)
    } else if handler.eq_ignore_ascii_case(b"XVID")
        || handler.eq_ignore_ascii_case(b"DIVX")
        || handler.eq_ignore_ascii_case(b"DX50")
        || handler.eq_ignore_ascii_case(b"MP4V")
        || handler.eq_ignore_ascii_case(b"FMP4")
    {
        Some(VideoCodec::Mpeg4)
    } else if handler.eq_ignore_ascii_case(b"MPEG")
        || handler.eq_ignore_ascii_case(b"MP2V")
        || handler.eq_ignore_ascii_case(b"MPG2")
    {
        Some(VideoCodec::Mpeg2)
    } else if handler.eq_ignore_ascii_case(b"MJPG") || handler.eq_ignore_ascii_case(b"MJPA") {
        Some(VideoCodec::Mjpeg)
    } else if handler.eq_ignore_ascii_case(b"VP80") {
        Some(VideoCodec::Vp8)
    } else if handler.eq_ignore_ascii_case(b"VP90") {
        Some(VideoCodec::Vp9)
    } else if handler.eq_ignore_ascii_case(b"AV01") {
        Some(VideoCodec::Av1)
    } else if handler.eq_ignore_ascii_case(b"WMV3") || handler.eq_ignore_ascii_case(b"WMV2") {
        Some(VideoCodec::Other("WMV".to_string()))
    } else {
        None
    }
}

fn avi_audio_handler(handler: &[u8]) -> Option<AudioCodec> {
    if handler.eq_ignore_ascii_case(b"MP3 ") || handler.eq_ignore_ascii_case(b"MP3") {
        Some(AudioCodec::Mp3)
    } else if handler.eq_ignore_ascii_case(b"AAC ") || handler.eq_ignore_ascii_case(b"AAC") {
        Some(AudioCodec::Aac)
    } else if handler.eq_ignore_ascii_case(b"FLAC") {
        Some(AudioCodec::Flac)
    } else if handler.eq_ignore_ascii_case(b"OPUS") {
        Some(AudioCodec::Opus)
    } else if handler == [0, 0, 0, 0] {
        None
    } else {
        let tag = u16::from_le_bytes([handler[0], handler[1]]);
        if tag == 0 {
            None
        } else {
            Some(codec_from_wave_tag(tag))
        }
    }
}

fn avi_audio_format(region: &[u8]) -> Option<AudioCodec> {
    let relative = find_subslice(region, b"strf")?;
    let body = relative + 8;
    if body + 2 > region.len() {
        return None;
    }

    let mut tag = le16(&region[body..body + 2]);
    if tag == 0xFFFE && body + 26 <= region.len() {
        tag = le16(&region[body + 24..body + 26]);
    }
    Some(codec_from_wave_tag(tag))
}

fn avi_video_compression(region: &[u8]) -> Option<&[u8]> {
    let relative = find_subslice(region, b"strf")?;
    let body = relative + 8;
    if body + 20 > region.len() {
        return None;
    }
    Some(&region[body + 16..body + 20])
}

fn detect_flv(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();

    let mut position = if data.len() >= 9 {
        be32(&data[5..9]) as usize
    } else {
        9
    };
    if position < 9 || position > data.len() {
        position = 9;
    }

    while position + 4 + 11 <= data.len() {
        position += 4; // PreviousTagSize
        if position + 11 > data.len() {
            break;
        }

        let tag_type = data[position];
        let data_size = be24(&data[position + 1..position + 4]);
        let payload_start = position + 11;
        let payload_end = payload_start.saturating_add(data_size).min(data.len());

        if payload_start < payload_end {
            let payload = &data[payload_start..payload_end];
            match tag_type {
                9 => {
                    let codec_id = payload[0] & 0x0F;
                    let codec = match codec_id {
                        7 => Some(VideoCodec::H264),
                        12 => Some(VideoCodec::H265),
                        13 => Some(VideoCodec::Av1),
                        4 | 5 => Some(VideoCodec::Vp6),
                        2 | 3 => Some(VideoCodec::H263),
                        _ => None,
                    };
                    if let Some(codec) = codec {
                        video_codecs.push(codec);
                    }
                }
                8 => {
                    let sound_format = payload[0] >> 4;
                    let codec = match sound_format {
                        0 | 3 => Some(AudioCodec::Pcm),
                        2 => Some(AudioCodec::Mp3),
                        10 => Some(AudioCodec::Aac),
                        11 => Some(AudioCodec::Speex),
                        14 => Some(AudioCodec::Opus),
                        _ => None,
                    };
                    if let Some(codec) = codec {
                        audio_codecs.push(codec);
                    }
                }
                _ => {}
            }
        }

        if data_size == 0 {
            position = payload_start;
        } else {
            position = payload_start.saturating_add(data_size);
        }
    }

    MediaFormat::build(
        Some(ContainerFormat::Flv),
        None,
        video_codecs,
        audio_codecs,
        extension,
    )
}

fn detect_ogg(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();

    if contains_subslice(data, b"OpusHead") {
        audio_codecs.push(AudioCodec::Opus);
    }
    if contains_subslice(data, b"\x01vorbis") {
        audio_codecs.push(AudioCodec::Vorbis);
    }
    if contains_subslice(data, b"\x7fFLAC") || contains_subslice(data, b"fLaC") {
        audio_codecs.push(AudioCodec::Flac);
    }
    if contains_subslice(data, b"Speex   ") {
        audio_codecs.push(AudioCodec::Speex);
    }
    if contains_subslice(data, b"\x80theora") {
        video_codecs.push(VideoCodec::Theora);
    }
    if contains_subslice(data, b"\x80vp8") {
        video_codecs.push(VideoCodec::Vp8);
    }
    if contains_subslice(data, b"\x80vp9") {
        video_codecs.push(VideoCodec::Vp9);
    }
    if contains_subslice(data, b"\x81av1") || contains_subslice(data, b"\x80av1") {
        video_codecs.push(VideoCodec::Av1);
    }

    let hint = if video_codecs.is_empty() && audio_codecs.is_empty() {
        AudioFormat::from_extension(extension.unwrap_or_default())
    } else {
        None
    };

    MediaFormat::build(
        Some(ContainerFormat::Ogg),
        hint,
        video_codecs,
        audio_codecs,
        extension,
    )
}

fn detect_ebml(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let document_type =
        find_ebml_string(data, &[0x42, 0x82]).map(|value| value.to_ascii_lowercase());

    let container = if document_type.as_deref() == Some("webm")
        || (document_type.is_none() && extension == Some("webm"))
    {
        ContainerFormat::WebM
    } else {
        ContainerFormat::Mkv
    };

    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();
    scan_ebml_codec_ids(data, &mut video_codecs, &mut audio_codecs);

    MediaFormat::build(Some(container), None, video_codecs, audio_codecs, extension)
}

fn read_ebml_size(data: &[u8], position: &mut usize) -> Option<usize> {
    let first = *data.get(*position)?;
    if first == 0 {
        return None;
    }

    let length = (first.leading_zeros() + 1) as usize;
    if length > 8 || *position + length > data.len() {
        return None;
    }

    let mask = if length >= 8 { 0 } else { 0xFFu8 >> length };
    let mut value = (first & mask) as u64;
    *position += 1;
    for _ in 1..length {
        value = (value << 8) | data[*position] as u64;
        *position += 1;
    }

    usize::try_from(value).ok()
}

fn find_ebml_string(data: &[u8], id: &[u8]) -> Option<String> {
    let mut search_from = 0usize;
    while search_from < data.len() {
        let relative = find_subslice(&data[search_from..], id)?;
        let id_position = search_from + relative;
        let mut position = id_position + id.len();

        if let Some(size) = read_ebml_size(data, &mut position)
            && size > 0
            && size <= 128
            && position + size <= data.len()
        {
            let value = &data[position..position + size];
            if value
                .iter()
                .all(|byte| *byte == 0 || byte.is_ascii_graphic())
            {
                let text = String::from_utf8_lossy(value)
                    .trim_end_matches('\0')
                    .to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }

        search_from = id_position + 1;
    }

    None
}

fn scan_ebml_codec_ids(
    data: &[u8],
    video_codecs: &mut Vec<VideoCodec>,
    audio_codecs: &mut Vec<AudioCodec>,
) {
    let mut position = 0usize;
    while position + 2 < data.len() {
        if data[position] != 0x86 {
            position += 1;
            continue;
        }

        let mut value_position = position + 1;
        let Some(size) = read_ebml_size(data, &mut value_position) else {
            position += 1;
            continue;
        };

        if (3..=64).contains(&size) && value_position + size <= data.len() {
            let codec_id = &data[value_position..value_position + size];
            if is_matroska_codec_id(codec_id) {
                map_matroska_codec(codec_id, video_codecs, audio_codecs);
                position = value_position + size;
                continue;
            }
        }

        position += 1;
    }
}

fn is_matroska_codec_id(value: &[u8]) -> bool {
    value
        .first()
        .is_some_and(|byte| *byte == b'V' || *byte == b'A')
        && value.iter().all(|byte| byte.is_ascii_graphic())
        && value.contains(&b'_')
}

fn map_matroska_codec(
    codec_id: &[u8],
    video_codecs: &mut Vec<VideoCodec>,
    audio_codecs: &mut Vec<AudioCodec>,
) {
    if codec_id.starts_with(b"V_MPEG4/ISO/AVC") {
        video_codecs.push(VideoCodec::H264);
    } else if codec_id.starts_with(b"V_MPEGH/ISO/HEVC") {
        video_codecs.push(VideoCodec::H265);
    } else if codec_id.starts_with(b"V_MPEG4/ISO/ASP")
        || codec_id.starts_with(b"V_MPEG4/ISO/SP")
        || codec_id.starts_with(b"V_MPEG4/MS/V3")
    {
        video_codecs.push(VideoCodec::Mpeg4);
    } else if codec_id.starts_with(b"V_MPEG2") {
        video_codecs.push(VideoCodec::Mpeg2);
    } else if codec_id.starts_with(b"V_MPEG1") {
        video_codecs.push(VideoCodec::Other("MPEG-1".to_string()));
    } else if codec_id.starts_with(b"V_VP8") {
        video_codecs.push(VideoCodec::Vp8);
    } else if codec_id.starts_with(b"V_VP9") {
        video_codecs.push(VideoCodec::Vp9);
    } else if codec_id.starts_with(b"V_AV1") {
        video_codecs.push(VideoCodec::Av1);
    } else if codec_id.starts_with(b"V_THEORA") {
        video_codecs.push(VideoCodec::Theora);
    } else if codec_id.starts_with(b"V_MJPEG") {
        video_codecs.push(VideoCodec::Mjpeg);
    } else if codec_id.starts_with(b"A_MPEG/L3") {
        audio_codecs.push(AudioCodec::Mp3);
    } else if codec_id.starts_with(b"A_MPEG/L2") {
        audio_codecs.push(AudioCodec::Mp2);
    } else if codec_id.starts_with(b"A_MPEG/L1") {
        audio_codecs.push(AudioCodec::Mp1);
    } else if codec_id.starts_with(b"A_AAC") {
        audio_codecs.push(AudioCodec::Aac);
    } else if codec_id.starts_with(b"A_FLAC") {
        audio_codecs.push(AudioCodec::Flac);
    } else if codec_id.starts_with(b"A_VORBIS") {
        audio_codecs.push(AudioCodec::Vorbis);
    } else if codec_id.starts_with(b"A_OPUS") {
        audio_codecs.push(AudioCodec::Opus);
    } else if codec_id.starts_with(b"A_AC3") {
        audio_codecs.push(AudioCodec::Ac3);
    } else if codec_id.starts_with(b"A_EAC3") {
        audio_codecs.push(AudioCodec::Eac3);
    } else if codec_id.starts_with(b"A_PCM") {
        audio_codecs.push(AudioCodec::Pcm);
    } else if codec_id.starts_with(b"A_WMA") {
        audio_codecs.push(AudioCodec::Wma);
    } else if codec_id.starts_with(b"A_ALAC") {
        audio_codecs.push(AudioCodec::Alac);
    } else if codec_id.starts_with(b"A_DTS") {
        audio_codecs.push(AudioCodec::Other("DTS".to_string()));
    }
}

fn detect_iso(data: &[u8], extension: Option<&str>) -> Result<MediaFormat> {
    let (container, brand) = iso_container_and_brand(data, extension);

    if let Some(brand) = brand
        && matches!(
            &brand,
            b"avif" | b"avis" | b"heic" | b"heix" | b"mif1" | b"msf1"
        )
    {
        return Err(OpenWaveError::unsupported_format(format!(
            "ISO base media brand '{}' is not audio or video",
            String::from_utf8_lossy(&brand)
        )));
    }

    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();
    collect_iso_codecs(data, &mut video_codecs, &mut audio_codecs);

    let hint = if video_codecs.is_empty() && audio_codecs.is_empty() {
        match brand {
            Some(brand) if &brand == b"M4A " => Some(AudioFormat::Aac),
            _ if matches!(extension, Some("m4a" | "m4b" | "m4p" | "aac")) => Some(AudioFormat::Aac),
            _ => None,
        }
    } else {
        None
    };

    Ok(MediaFormat::build(
        Some(container),
        hint,
        video_codecs,
        audio_codecs,
        extension,
    ))
}

fn iso_container_and_brand(
    data: &[u8],
    extension: Option<&str>,
) -> (ContainerFormat, Option<[u8; 4]>) {
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        let brand = [data[8], data[9], data[10], data[11]];
        let container = if &brand == b"qt  " {
            ContainerFormat::Mov
        } else {
            ContainerFormat::Mp4
        };
        return (container, Some(brand));
    }

    let container = match extension {
        Some("mp4" | "m4v" | "m4p" | "m4a" | "m4b") => ContainerFormat::Mp4,
        _ => ContainerFormat::Mov,
    };
    (container, None)
}

fn is_iso_bmff(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }

    let size = be32(&data[0..4]);
    let box_type = &data[4..8];
    let plausible_size = size == 0 || size >= 8;

    (box_type == b"ftyp" && plausible_size)
        || (matches!(
            box_type,
            b"moov" | b"mdat" | b"wide" | b"free" | b"skip" | b"pnot"
        ) && plausible_size)
}

fn collect_iso_codecs(
    data: &[u8],
    video_codecs: &mut Vec<VideoCodec>,
    audio_codecs: &mut Vec<AudioCodec>,
) {
    walk_iso_boxes(data, 0, data.len(), video_codecs, audio_codecs, 0);
}

fn walk_iso_boxes(
    data: &[u8],
    mut position: usize,
    end: usize,
    video_codecs: &mut Vec<VideoCodec>,
    audio_codecs: &mut Vec<AudioCodec>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }

    while position + 8 <= end {
        let size32 = be32(&data[position..position + 4]);
        let box_type = &data[position + 4..position + 8];

        let (size, header_size) = if size32 == 1 {
            if position + 16 > end {
                return;
            }
            (be64(&data[position + 8..position + 16]), 16usize)
        } else if size32 == 0 {
            ((end - position) as u64, 8usize)
        } else {
            (size32 as u64, 8usize)
        };

        let Ok(size) = usize::try_from(size) else {
            return;
        };
        if size < header_size || position.saturating_add(size) > end {
            return;
        }

        let payload_start = position + header_size;
        let payload_end = position + size;

        match box_type {
            b"stsd" => parse_stsd(data, payload_start, payload_end, video_codecs, audio_codecs),
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" => walk_iso_boxes(
                data,
                payload_start,
                payload_end,
                video_codecs,
                audio_codecs,
                depth + 1,
            ),
            _ => {}
        }

        position = payload_end;
    }
}

fn parse_stsd(
    data: &[u8],
    start: usize,
    end: usize,
    video_codecs: &mut Vec<VideoCodec>,
    audio_codecs: &mut Vec<AudioCodec>,
) {
    if start + 8 > end {
        return;
    }

    let entry_count = be32(&data[start + 4..start + 8]) as usize;
    let mut position = start + 8;

    for _ in 0..entry_count.min(128) {
        if position + 8 > end {
            break;
        }

        let size = be32(&data[position..position + 4]) as usize;
        let entry_type = &data[position + 4..position + 8];
        if size < 8 || position.saturating_add(size) > end {
            break;
        }

        let entry_end = position + size;
        map_sample_entry(
            entry_type,
            &data[position + 8..entry_end],
            video_codecs,
            audio_codecs,
        );
        position = entry_end;
    }
}

fn map_sample_entry(
    entry_type: &[u8],
    body: &[u8],
    video_codecs: &mut Vec<VideoCodec>,
    audio_codecs: &mut Vec<AudioCodec>,
) {
    match entry_type {
        b"avc1" | b"avc3" => video_codecs.push(VideoCodec::H264),
        b"hvc1" | b"hev1" => video_codecs.push(VideoCodec::H265),
        b"vp08" => video_codecs.push(VideoCodec::Vp8),
        b"vp09" => video_codecs.push(VideoCodec::Vp9),
        b"av01" => video_codecs.push(VideoCodec::Av1),
        b"mp4v" => video_codecs.push(VideoCodec::Mpeg4),
        b"mp2v" | b"mpg2" | b"m2v1" => video_codecs.push(VideoCodec::Mpeg2),
        b"mjpa" | b"mjpb" | b"jpeg" => video_codecs.push(VideoCodec::Mjpeg),
        b"mp4a" => audio_codecs.push(mp4a_audio_codec(body)),
        b"ac-3" => audio_codecs.push(AudioCodec::Ac3),
        b"ec-3" => audio_codecs.push(AudioCodec::Eac3),
        b"Opus" => audio_codecs.push(AudioCodec::Opus),
        b"fLaC" => audio_codecs.push(AudioCodec::Flac),
        b"alac" => audio_codecs.push(AudioCodec::Alac),
        b".mp3" | b"mp3 " => audio_codecs.push(AudioCodec::Mp3),
        b"wma " => audio_codecs.push(AudioCodec::Wma),
        b"sowt" | b"twos" | b"lpcm" | b"raw " => audio_codecs.push(AudioCodec::Pcm),
        b"samr" | b"sawb" => audio_codecs.push(AudioCodec::Other("AMR".to_string())),
        _ => {}
    }
}

fn mp4a_audio_codec(body: &[u8]) -> AudioCodec {
    let Some(esds) = find_subslice(body, b"esds") else {
        return AudioCodec::Aac;
    };

    let mut position = esds + 8;
    while position + 2 < body.len() {
        if body[position] == 0x04 {
            let mut descriptor = position + 1;
            if let Some(length) = read_descriptor_length(body, &mut descriptor)
                && length > 0
                && descriptor < body.len()
            {
                return codec_from_mpeg4_object_type(body[descriptor]);
            }
        }
        position += 1;
    }

    AudioCodec::Aac
}

fn read_descriptor_length(data: &[u8], position: &mut usize) -> Option<usize> {
    let mut length = 0usize;
    for _ in 0..4 {
        let byte = *data.get(*position)?;
        *position += 1;
        length = (length << 7) | (byte as usize & 0x7F);
        if byte & 0x80 == 0 {
            return Some(length);
        }
    }
    Some(length)
}

fn codec_from_mpeg4_object_type(object_type: u8) -> AudioCodec {
    match object_type {
        0x40 | 0x66 | 0x67 | 0x68 => AudioCodec::Aac,
        0x69 | 0x6B => AudioCodec::Mp3,
        0xA5 => AudioCodec::Ac3,
        0xA6 => AudioCodec::Eac3,
        0xAD => AudioCodec::Opus,
        0xDD => AudioCodec::Vorbis,
        _ => AudioCodec::Aac,
    }
}

fn is_mpeg_program_stream(data: &[u8]) -> bool {
    data.len() >= 4
        && data[0] == 0
        && data[1] == 0
        && data[2] == 1
        && matches!(data[3], 0xB3 | 0xBA | 0xBB)
}

fn is_mpeg_transport_stream(data: &[u8], extension: Option<&str>) -> bool {
    if data.first() != Some(&0x47) {
        return false;
    }

    if data.len() > 376 && data[188] == 0x47 && data[376] == 0x47 {
        return true;
    }

    if data.len() > 188 && data[188] == 0x47 {
        return true;
    }

    matches!(extension, Some("ts" | "m2ts" | "mts")) && data.len() >= 16
}

fn detect_mpeg(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();

    if contains_subslice(data, &[0x00, 0x00, 0x01, 0xB3]) {
        video_codecs.push(VideoCodec::Mpeg2);
    }

    if contains_subslice(data, &[0x00, 0x00, 0x00, 0x01, 0x67])
        || contains_subslice(data, &[0x00, 0x00, 0x01, 0x67])
    {
        video_codecs.push(VideoCodec::H264);
    }

    if contains_subslice(data, &[0x00, 0x00, 0x00, 0x01, 0x42, 0x01])
        || contains_subslice(data, &[0x00, 0x00, 0x01, 0x42, 0x01])
    {
        video_codecs.push(VideoCodec::H265);
    }

    if contains_subslice(data, &[0x0B, 0x77]) {
        audio_codecs.push(AudioCodec::Ac3);
    }

    let mut found_adts = false;
    let mut found_mpeg_audio = false;
    for position in 0..data.len() {
        if data[position] != 0xFF {
            continue;
        }

        if !found_adts && is_adts(&data[position..]) {
            audio_codecs.push(AudioCodec::Aac);
            found_adts = true;
        } else if !found_mpeg_audio && let Some(codec) = mpeg_audio_codec_at(data, position) {
            audio_codecs.push(codec);
            found_mpeg_audio = true;
        }

        if found_adts && found_mpeg_audio {
            break;
        }
    }

    MediaFormat::build(
        Some(ContainerFormat::Mpeg),
        None,
        video_codecs,
        audio_codecs,
        extension,
    )
}

fn scan_iso_file(
    file: &mut File,
    file_len: u64,
) -> std::io::Result<(Vec<VideoCodec>, Vec<AudioCodec>)> {
    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();
    let mut position = 0u64;

    while position + 8 <= file_len {
        file.seek(SeekFrom::Start(position))?;
        let mut header = [0u8; 16];
        let read = read_up_to(file, &mut header)?;
        if read < 8 {
            break;
        }

        let size32 = be32(&header[0..4]);
        let box_type = &header[4..8];
        let (box_size, header_size) = if size32 == 1 {
            if read < 16 {
                break;
            }
            (be64(&header[8..16]), 16u64)
        } else if size32 == 0 {
            (file_len - position, 8u64)
        } else {
            (size32 as u64, 8u64)
        };

        if box_size < header_size || position.saturating_add(box_size) > file_len {
            break;
        }

        if box_type == b"moov" {
            let payload_size = box_size - header_size;
            if payload_size <= MAX_MOOV_BYTES {
                file.seek(SeekFrom::Start(position + header_size))?;
                let mut moov = vec![0u8; payload_size as usize];
                file.read_exact(&mut moov)?;
                collect_iso_codecs(&moov, &mut video_codecs, &mut audio_codecs);
            }
            break;
        }

        position += box_size;
    }

    Ok((video_codecs, audio_codecs))
}

fn read_up_to(file: &mut File, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0usize;
    while total < buffer.len() {
        match file.read(&mut buffer[total..])? {
            0 => break,
            read => total += read,
        }
    }
    Ok(total)
}

fn detect_asf(data: &[u8], extension: Option<&str>) -> MediaFormat {
    let has_video =
        contains_subslice(data, &ASF_VIDEO_MEDIA_GUID) || matches!(extension, Some("wmv"));

    let mut video_codecs = Vec::new();
    let mut audio_codecs = Vec::new();
    let mut audio_format = None;

    if has_video {
        video_codecs.push(VideoCodec::Other("WMV".to_string()));
    } else {
        audio_codecs.push(AudioCodec::Wma);
        audio_format = Some(AudioFormat::Wma);
    }

    MediaFormat::build(
        Some(ContainerFormat::Asf),
        audio_format,
        video_codecs,
        audio_codecs,
        extension,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    fn iso_box(box_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + payload.len());
        bytes.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
        bytes.extend_from_slice(box_type);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn mp4_fixture(video_entry: Option<&[u8; 4]>, audio_entry: Option<&[u8; 4]>) -> Vec<u8> {
        let mut entries = Vec::new();
        if let Some(entry) = video_entry {
            entries.extend_from_slice(&iso_box(entry, &[0u8; 16]));
        }
        if let Some(entry) = audio_entry {
            entries.extend_from_slice(&iso_box(entry, &[0u8; 16]));
        }

        let entry_count = usize::from(video_entry.is_some()) + usize::from(audio_entry.is_some());
        let mut stsd_payload = vec![0u8; 4];
        stsd_payload.extend_from_slice(&(entry_count as u32).to_be_bytes());
        stsd_payload.extend_from_slice(&entries);

        let stsd = iso_box(b"stsd", &stsd_payload);
        let stbl = iso_box(b"stbl", &stsd);
        let minf = iso_box(b"minf", &stbl);
        let mdia = iso_box(b"mdia", &minf);
        let trak = iso_box(b"trak", &mdia);
        let moov = iso_box(b"moov", &trak);

        let mut ftyp_payload = Vec::new();
        ftyp_payload.extend_from_slice(b"isom");
        ftyp_payload.extend_from_slice(&512u32.to_be_bytes());
        ftyp_payload.extend_from_slice(b"isomiso2mp41");

        let mut file = iso_box(b"ftyp", &ftyp_payload);
        file.extend_from_slice(&moov);
        file
    }

    fn ebml_element(id: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(id);
        let length = payload.len();
        bytes.push(0x80 | length as u8);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn mkv_fixture(document_type: &str, codec_ids: &[&str]) -> Vec<u8> {
        let mut bytes = EBML_MAGIC.to_vec();
        bytes.extend_from_slice(&ebml_element(&[0x42, 0x82], document_type.as_bytes()));
        for codec_id in codec_ids {
            bytes.extend_from_slice(&ebml_element(&[0x86], codec_id.as_bytes()));
        }
        bytes
    }

    fn wav_fixture(format_tag: u16) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&format_tag.to_le_bytes());
        fmt.extend_from_slice(&2u16.to_le_bytes());
        fmt.extend_from_slice(&44_100u32.to_le_bytes());
        fmt.extend_from_slice(&176_400u32.to_le_bytes());
        fmt.extend_from_slice(&4u16.to_le_bytes());
        fmt.extend_from_slice(&16u16.to_le_bytes());

        let mut data = b"RIFF".to_vec();
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"WAVE");
        data.extend_from_slice(b"fmt ");
        data.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        data.extend_from_slice(&fmt);
        data.extend_from_slice(b"data");
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]);
        let riff_size = (data.len() - 8) as u32;
        data[4..8].copy_from_slice(&riff_size.to_le_bytes());
        data
    }

    fn flv_fixture() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"FLV");
        data.push(1);
        data.push(0x05);
        data.extend_from_slice(&9u32.to_be_bytes());

        // Video tag with AVC sequence header.
        data.extend_from_slice(&0u32.to_be_bytes());
        data.push(9);
        data.extend_from_slice(&[0, 0, 5]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0x17, 0x00, 0, 0, 0]);

        // Audio tag with AAC sequence header.
        data.extend_from_slice(&0u32.to_be_bytes());
        data.push(8);
        data.extend_from_slice(&[0, 0, 2]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0xAF, 0x00]);

        data
    }

    fn asf_fixture() -> Vec<u8> {
        let mut data = ASF_HEADER_GUID.to_vec();
        data.extend_from_slice(&[0u8; 64]);
        data
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("openwave-{name}-{nonce}"))
    }

    #[test]
    fn detects_mp4_video_and_audio_codecs() {
        let data = mp4_fixture(Some(b"avc1"), Some(b"mp4a"));
        let format = FormatDetector::new()
            .detect_bytes(&data, Some("mp4"))
            .unwrap();

        assert_eq!(format.container, Some(ContainerFormat::Mp4));
        assert_eq!(format.kind, MediaKind::AudioVideo);
        assert!(format.video_codecs.contains(&VideoCodec::H264));
        assert!(format.audio_codecs.contains(&AudioCodec::Aac));
    }

    #[test]
    fn detects_mov_brand() {
        let mut ftyp_payload = b"qt  ".to_vec();
        ftyp_payload.extend_from_slice(&[0u8; 8]);
        let ftyp = iso_box(b"ftyp", &ftyp_payload);
        let format = FormatDetector::new()
            .detect_bytes(&ftyp, Some("mov"))
            .unwrap();

        assert_eq!(format.container, Some(ContainerFormat::Mov));
    }

    #[test]
    fn detects_matroska_and_webm() {
        let detector = FormatDetector::new();
        let mkv = mkv_fixture("matroska", &["V_MPEG4/ISO/AVC", "A_AAC", "A_OPUS"]);
        let format = detector.detect_bytes(&mkv, Some("mkv")).unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Mkv));
        assert!(format.video_codecs.contains(&VideoCodec::H264));
        assert!(format.audio_codecs.contains(&AudioCodec::Aac));
        assert!(format.audio_codecs.contains(&AudioCodec::Opus));

        let webm = mkv_fixture("webm", &["V_VP9", "A_OPUS"]);
        let format = detector.detect_bytes(&webm, Some("webm")).unwrap();
        assert_eq!(format.container, Some(ContainerFormat::WebM));
        assert_eq!(format.kind, MediaKind::AudioVideo);
        assert_eq!(format.video_codecs, vec![VideoCodec::Vp9]);
        assert_eq!(format.audio_codecs, vec![AudioCodec::Opus]);
    }

    #[test]
    fn detects_avi_audio_and_video_handlers() {
        let mut data = b"RIFF".to_vec();
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(b"AVI ");

        data.extend_from_slice(b"strh");
        data.extend_from_slice(&56u32.to_le_bytes());
        data.extend_from_slice(b"vidsH264");
        data.extend_from_slice(&[0u8; 48]);

        data.extend_from_slice(b"strh");
        data.extend_from_slice(&56u32.to_le_bytes());
        data.extend_from_slice(b"auds\0\0\0\0");
        data.extend_from_slice(b"strf");
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&0x0055u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 14]);

        let format = FormatDetector::new()
            .detect_bytes(&data, Some("avi"))
            .unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Avi));
        assert!(format.video_codecs.contains(&VideoCodec::H264));
        assert!(format.audio_codecs.contains(&AudioCodec::Mp3));
    }

    #[test]
    fn detects_flv_tags() {
        let format = FormatDetector::new()
            .detect_bytes(&flv_fixture(), Some("flv"))
            .unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Flv));
        assert_eq!(format.video_codecs, vec![VideoCodec::H264]);
        assert_eq!(format.audio_codecs, vec![AudioCodec::Aac]);
    }

    #[test]
    fn detects_ogg_vorbis_and_opus() {
        let detector = FormatDetector::new();

        let mut vorbis = b"OggS".to_vec();
        vorbis.extend_from_slice(
            b"\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01vorbis",
        );
        let format = detector.detect_bytes(&vorbis, Some("ogg")).unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Ogg));
        assert_eq!(format.audio_format, Some(AudioFormat::Ogg));
        assert_eq!(format.audio_codecs, vec![AudioCodec::Vorbis]);

        let mut opus = b"OggS".to_vec();
        opus.extend_from_slice(b"\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00OpusHead");
        let format = detector.detect_bytes(&opus, Some("opus")).unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Ogg));
        assert_eq!(format.audio_format, Some(AudioFormat::Opus));
        assert_eq!(format.kind, MediaKind::Audio);
    }

    #[test]
    fn detects_mpeg_program_stream() {
        let mut data = vec![0, 0, 1, 0xBA];
        data.extend_from_slice(&[0u8; 16]);
        data.extend_from_slice(&[0, 0, 1, 0xB3]);
        data.extend_from_slice(&[0u8; 16]);
        data.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x64]);

        let format = FormatDetector::new()
            .detect_bytes(&data, Some("mpg"))
            .unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Mpeg));
        assert!(format.video_codecs.contains(&VideoCodec::Mpeg2));
        assert!(format.audio_codecs.contains(&AudioCodec::Mp3));
    }

    #[test]
    fn detects_standalone_audio_formats() {
        let detector = FormatDetector::new();

        assert_eq!(
            detector
                .detect_bytes(b"fLaC\0\0\0\0", Some("flac"))
                .unwrap()
                .audio_format,
            Some(AudioFormat::Flac)
        );

        assert_eq!(
            detector
                .detect_bytes(&[0xFF, 0xFB, 0x90, 0x64], Some("mp3"))
                .unwrap()
                .audio_format,
            Some(AudioFormat::Mp3)
        );

        assert_eq!(
            detector
                .detect_bytes(&[0xFF, 0xF1, 0x50, 0x80, 0, 0, 0], Some("aac"))
                .unwrap()
                .audio_format,
            Some(AudioFormat::Aac)
        );

        assert_eq!(
            detector
                .detect_bytes(&wav_fixture(0x0001), Some("wav"))
                .unwrap()
                .audio_format,
            Some(AudioFormat::Wav)
        );

        assert_eq!(
            detector
                .detect_bytes(&asf_fixture(), Some("wma"))
                .unwrap()
                .audio_format,
            Some(AudioFormat::Wma)
        );

        let id3 = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
        assert_eq!(
            detector
                .detect_bytes(&id3, Some("mp3"))
                .unwrap()
                .audio_format,
            Some(AudioFormat::Mp3)
        );
    }

    #[test]
    fn rejects_unrecognised_content() {
        let error = FormatDetector::new()
            .detect_bytes(b"definitely not a media file", Some("mp4"))
            .unwrap_err();
        assert!(error.is_unsupported_format());
        assert!(error.to_string().contains("unsupported media format"));
    }

    #[test]
    fn detection_is_within_budget() {
        let data = mp4_fixture(Some(b"avc1"), Some(b"mp4a"));
        let started = Instant::now();
        let _ = FormatDetector::new()
            .detect_bytes(&data, Some("mp4"))
            .unwrap();
        assert!(started.elapsed().as_millis() < 500);
    }

    #[test]
    fn reads_moov_after_probe_window_from_file() {
        // Insert a large mdat before the moov atom, then rebuild with codecs.
        let with_codecs = mp4_fixture(Some(b"hvc1"), Some(b"ac-3"));
        let ftyp_end = be32(&with_codecs[0..4]) as usize;
        let ftyp = &with_codecs[..ftyp_end];
        let moov = &with_codecs[ftyp_end..];

        let mut data = ftyp.to_vec();
        let mdat_payload = vec![0u8; 256];
        data.extend_from_slice(&iso_box(b"mdat", &mdat_payload));
        data.extend_from_slice(moov);

        let path = temp_path("moov.mp4");
        fs::write(&path, &data).unwrap();

        let detector = FormatDetector::new().with_max_probe_bytes(64);
        let result = detector.detect(&path);
        fs::remove_file(&path).unwrap();

        let format = result.unwrap();
        assert_eq!(format.container, Some(ContainerFormat::Mp4));
        assert!(format.video_codecs.contains(&VideoCodec::H265));
        assert!(format.audio_codecs.contains(&AudioCodec::Ac3));
    }

    #[test]
    fn detect_from_file_reports_path_on_unsupported_format() {
        let path = temp_path("unknown.bin");
        fs::write(&path, b"not media").unwrap();

        let error = FormatDetector::new().detect(&path).unwrap_err();
        fs::remove_file(&path).unwrap();

        assert!(error.is_unsupported_format());
        assert!(error.to_string().contains(&path.display().to_string()));
    }
}
