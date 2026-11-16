use sdl3::{
    audio::{AudioSpec, AudioSpecWAV, AudioStreamOwner},
    iostream::IOStream,
};

/// The original Happy Years / 1984 camera sound, embedded without resampling.
pub struct Shutter {
    stream: AudioStreamOwner,
    wav: AudioSpecWAV,
}

impl Shutter {
    pub fn new() -> Result<Self, sdl3::Error> {
        let audio = sdl3::init()?.audio()?;
        let wav = AudioSpecWAV::load_wav_rw(&mut IOStream::from_bytes(include_bytes!(
            "../assets/shutter.wav"
        ))?)?;
        let spec = AudioSpec {
            freq: Some(wav.freq),
            channels: Some(i32::from(wav.channels)),
            format: Some(wav.format),
        };
        let stream = audio
            .open_playback_device(&spec)?
            .open_device_stream(Some(&spec))?;
        Ok(Self { stream, wav })
    }

    pub fn play(&self) -> Result<(), sdl3::Error> {
        self.stream.clear()?;
        self.stream.put_data(self.wav.buffer())?;
        self.stream.flush()?;
        self.stream.resume()
    }
}
