//! Minimal file output example; the library itself only returns PCM samples.
use shtts::{VoiceSettings, synthesize};
use std::{
    env,
    fs::File,
    io::{self, Write},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let text = args
        .next()
        .ok_or_else(|| io::Error::other("usage: synthesize <katakana> <output.pcm>"))?;
    let path = args
        .next()
        .ok_or_else(|| io::Error::other("missing output PCM path"))?;
    let pcm = synthesize(&text, &VoiceSettings::default())?;
    let mut file = io::BufWriter::new(File::create(path)?);
    for sample in pcm {
        file.write_all(&sample.to_le_bytes())?;
    }
    file.flush()?;
    Ok(())
}
