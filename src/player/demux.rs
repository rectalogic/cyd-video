extern crate alloc;

use super::audio;
use crate::error::Error;
use alloc::{boxed::Box, collections::VecDeque, vec::Vec};
use embassy_time::Duration;
use embedded_io::{Read, Seek};
use riffparse::{Chunk, EmbeddedAdapter, Riff, RiffParser, avi, fourcc};
use thiserror::Error;

mod tag {
    use super::fourcc;
    pub const MJPG: fourcc::Fourcc = fourcc::Fourcc::new(*b"MJPG");
}

#[derive(Error, Debug)]
pub enum DemuxError {
    #[error("No AVI video stream found")]
    NoVideo,
    #[error("Invalid AVI video stream {0} found")]
    InvalidVideoStream(fourcc::Fourcc),
    #[error("Invalid AVI audio stream {0:?} found")]
    InvalidAudioStream(avi::WaveFormat),
}

type ChunkIter<R> = avi::ChunkIter<EmbeddedAdapter<R>, Box<dyn FnMut(fourcc::Fourcc) -> bool>>;

pub struct Demuxer<R> {
    avi_parser: avi::AviParser<EmbeddedAdapter<R>>,
    frame_duration: Duration,
    video_chunks: ChunkCache,
    audio_chunks: ChunkCache,
    chunk_iter: ChunkIter<R>,
}

impl<R> Demuxer<R>
where
    R: Read + Seek,
{
    pub fn new(reader: R) -> Result<Self, Error> {
        let avi_parser = avi::AviParser::new(RiffParser::new(EmbeddedAdapter(reader)))?;
        let Some(video_stream) = avi_parser.find_best_stream::<avi::VideoStream>() else {
            return Err(Error::Demux(DemuxError::NoVideo));
        };
        if !matches!(video_stream.stream_header.fcc_handler, tag::MJPG) {
            return Err(Error::Demux(DemuxError::InvalidVideoStream(
                video_stream.stream_header.fcc_handler,
            )));
        }
        let video_stream_id = video_stream.stream_id;

        let audio_stream_id =
            if let Some(audio_stream) = avi_parser.find_best_stream::<avi::AudioStream>() {
                const CHANNELS: u16 = audio::SAMPLE_CHANNELS as u16;
                if !matches!(
                    audio_stream.wave_format,
                    avi::WaveFormat::Pcm(avi::WaveFormatEx {
                        channels: CHANNELS,
                        samples_per_sec: audio::SAMPLE_RATE,
                        bits_per_sample: audio::SAMPLE_DATA_FORMAT,
                        ..
                    })
                ) {
                    return Err(Error::Demux(DemuxError::InvalidAudioStream(
                        audio_stream.wave_format.clone(),
                    )));
                }
                audio_stream.stream_id
            } else {
                fourcc::tag::NULL
            };

        let frame_duration =
            Duration::from_micros(avi_parser.avi_header.micro_sec_per_frame as u64);

        let filter: Box<dyn FnMut(fourcc::Fourcc) -> bool> =
            Box::new(move |id| video_stream_id == id || audio_stream_id == id);
        let chunk_iter = avi_parser.movi_chunks(filter);

        Ok(Self {
            avi_parser,
            frame_duration,
            video_chunks: ChunkCache::new(video_stream_id),
            audio_chunks: ChunkCache::new(audio_stream_id),
            chunk_iter,
        })
    }

    pub fn frame_duration(&self) -> Duration {
        self.frame_duration
    }

    pub fn next_video_chunk(&mut self) -> Option<Result<Riff<Chunk>, Error>> {
        self.video_chunks
            .next(&mut self.chunk_iter, &mut self.audio_chunks)
    }

    pub fn next_audio_chunk(&mut self) -> Option<Result<Riff<Chunk>, Error>> {
        self.audio_chunks
            .next(&mut self.chunk_iter, &mut self.video_chunks)
    }

    pub fn read_chunk_data(&self, chunk: Riff<Chunk>, buffer: &mut Vec<u8>) -> Result<(), Error> {
        self.avi_parser
            .riff_parser()
            .read_data_vec(chunk, buffer)
            .map_err(Error::BinRead)
    }
}

struct ChunkCache {
    stream_id: fourcc::Fourcc,
    chunks: VecDeque<Riff<Chunk>>,
}

impl ChunkCache {
    fn new(stream_id: fourcc::Fourcc) -> Self {
        Self {
            stream_id,
            chunks: VecDeque::with_capacity(2),
        }
    }

    fn next<R>(
        &mut self,
        chunks: &mut ChunkIter<R>,
        other: &mut ChunkCache,
    ) -> Option<Result<Riff<Chunk>, Error>>
    where
        R: Read + Seek,
    {
        if let Some(chunk) = self.chunks.pop_front() {
            return Some(Ok(chunk));
        }
        loop {
            match chunks.next()? {
                Ok(chunk) if chunk.id() == self.stream_id => return Some(Ok(chunk)),
                Ok(chunk) => other.chunks.push_back(chunk),
                Err(e) => return Some(Err(Error::BinRead(e))),
            }
        }
    }
}
