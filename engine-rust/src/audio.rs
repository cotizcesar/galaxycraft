//! Retail sound effects and a small software mixer.
//!
//! The shipped `sound\*.wav` files are ordinary 44.1 kHz 16-bit PCM.  The
//! bank in `assets/sound` is extracted byte-for-byte from CASC; the player
//! mixes overlapping effects into one mono stream and hands it to the
//! platform output (ALSA, WinMM or CoreAudio, all loaded at run time).  When
//! no audio device exists (headless runs, CI) the player degrades to a no-op
//! so gameplay is unaffected.

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

pub const SAMPLE_RATE: u32 = 44_100;

/// One decoded PCM WAV file.  `samples` is interleaved at the source rate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wav {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

impl Wav {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let mut fmt: Option<(u16, u16, u32, u16)> = None;
        let mut data: Option<&[u8]> = None;
        let mut pos = 12usize;
        while pos + 8 <= bytes.len() {
            let id = &bytes[pos..pos + 4];
            let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
            let start = pos + 8;
            let end = start.checked_add(size)?;
            if end > bytes.len() {
                break;
            }
            match id {
                b"fmt " if size >= 16 => {
                    let tag = u16::from_le_bytes([bytes[start], bytes[start + 1]]);
                    let channels = u16::from_le_bytes([bytes[start + 2], bytes[start + 3]]);
                    let rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().ok()?);
                    let bits = u16::from_le_bytes([bytes[start + 14], bytes[start + 15]]);
                    fmt = Some((tag, channels, rate, bits));
                }
                b"data" => data = Some(&bytes[start..end]),
                _ => {}
            }
            pos = end + (size & 1);
        }
        let (tag, channels, rate, bits) = fmt?;
        // 1 = PCM; 3 = IEEE float is not used by the shipped effects.
        if tag != 1 || channels == 0 || rate == 0 {
            return None;
        }
        let raw = data?;
        let mut samples = Vec::new();
        match bits {
            8 => {
                samples.extend(raw.iter().map(|&b| (b as i16 - 128) * 256));
            }
            16 => {
                for pair in raw.as_chunks::<2>().0 {
                    samples.push(i16::from_le_bytes([pair[0], pair[1]]));
                }
            }
            24 => {
                for triple in raw.as_chunks::<3>().0 {
                    let v =
                        ((triple[2] as i32) << 16) | ((triple[1] as i32) << 8) | triple[0] as i32;
                    samples.push((v >> 8) as i16);
                }
            }
            32 => {
                for quad in raw.as_chunks::<4>().0 {
                    samples.push(i32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]) as i16);
                }
            }
            _ => return None,
        }
        Some(Self {
            rate,
            channels,
            samples,
        })
    }

    /// Duration in seconds at the source rate.
    pub fn seconds(&self) -> f32 {
        let frames = self.samples.len() as f32 / self.channels.max(1) as f32;
        frames / self.rate.max(1) as f32
    }

    /// Resample to the mixer's mono 44.1 kHz stream.
    pub fn to_mono_44k(&self) -> Vec<i16> {
        let ch = self.channels.max(1) as usize;
        let frames = self.samples.len() / ch;
        if frames == 0 {
            return Vec::new();
        }
        let mono: Vec<i16> = (0..frames)
            .map(|f| {
                let sum: i32 = (0..ch).map(|c| self.samples[f * ch + c] as i32).sum();
                (sum / ch as i32) as i16
            })
            .collect();
        if self.rate == SAMPLE_RATE {
            return mono;
        }
        let ratio = SAMPLE_RATE as f64 / self.rate as f64;
        let out_len = ((frames as f64) * ratio).round() as usize;
        let mut out = Vec::with_capacity(out_len);
        for i in 0..out_len {
            let src = i as f64 / ratio;
            let a = src.floor() as usize;
            let t = (src - a as f64) as f32;
            let s0 = mono[a.min(frames - 1)] as f32;
            let s1 = mono[(a + 1).min(frames - 1)] as f32;
            out.push((s0 + (s1 - s0) * t).round().clamp(-32768.0, 32767.0) as i16);
        }
        out
    }
}

/// The bundled effects.  Names match `assets/sound/<name>.wav`.
pub const SOUND_NAMES: &[&str] = &[
    "ui_click",
    "ui_move",
    "ui_swish",
    "ui_swish_out",
    "rescue_terran",
    "rescue_zerg",
    "rescue_protoss",
    "transmission",
    "terran_yes",
    "zerg_yes",
    "protoss_yes",
    "victory",
    "defeat",
];

/// Decoded effect bank.  Missing files are simply absent, which keeps the
/// client usable with a partial asset directory.
#[derive(Default)]
pub struct SoundBank {
    sounds: HashMap<String, Vec<i16>>,
}

impl SoundBank {
    pub fn load(dir: &Path) -> Self {
        let mut sounds = HashMap::new();
        for name in SOUND_NAMES {
            let path = dir.join(format!("{name}.wav"));
            if let Ok(bytes) = std::fs::read(&path)
                && let Some(wav) = Wav::parse(&bytes)
            {
                sounds.insert((*name).to_string(), wav.to_mono_44k());
            }
        }
        Self { sounds }
    }

    pub fn len(&self) -> usize {
        self.sounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    pub fn has(&self, name: &str) -> bool {
        self.sounds.contains_key(name)
    }

    pub fn get(&self, name: &str) -> Option<&[i16]> {
        self.sounds.get(name).map(|v| v.as_slice())
    }

    /// Race acknowledgement used when a unit accepts an order.
    pub fn race_yes(&self, race: crate::sim::Race) -> Option<&[i16]> {
        let name = match race {
            crate::sim::Race::Zerg => "zerg_yes",
            crate::sim::Race::Protoss => "protoss_yes",
            _ => "terran_yes",
        };
        self.get(name)
    }
}

/// Decode a map sound (PCM `.wav` or Remastered `.ogg`) into the mixer's
/// mono 44.1 kHz stream; `None` for anything else.
pub fn decode_sound(bytes: &[u8]) -> Option<Vec<i16>> {
    match Wav::parse(bytes) {
        Some(wav) => Some(wav.to_mono_44k()),
        None if bytes.starts_with(b"OggS") => decode_ogg(bytes),
        None => None,
    }
}

/// Decode an Ogg Vorbis track (the Remastered `music/*.ogg` set) into the
/// mixer's mono 44.1 kHz stream.
///
/// Each Ogg page is fed through `lewton`'s Vorbis decoder; the returned
/// samples are interleaved at the source rate and then resampled exactly like
/// a WAV clip.
pub fn decode_ogg(bytes: &[u8]) -> Option<Vec<i16>> {
    use std::io::Cursor;
    let mut reader = lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes.to_vec())).ok()?;
    let channels = reader.ident_hdr.audio_channels.max(1) as usize;
    let rate = reader.ident_hdr.audio_sample_rate;
    if rate == 0 {
        return None;
    }
    let mut samples: Vec<i16> = Vec::new();
    loop {
        match reader.read_dec_packet_itl() {
            Ok(Some(packet)) => samples.extend_from_slice(&packet),
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    if samples.is_empty() {
        return None;
    }
    let frames = samples.len() / channels;
    let mono: Vec<i16> = (0..frames)
        .map(|f| {
            let sum: i32 = (0..channels)
                .map(|c| samples[f * channels + c] as i32)
                .sum();
            (sum / channels as i32) as i16
        })
        .collect();
    if rate == SAMPLE_RATE {
        return Some(mono);
    }
    let ratio = SAMPLE_RATE as f64 / rate as f64;
    let out_len = ((frames as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let a = src.floor() as usize;
        let t = (src - a as f64) as f32;
        let s0 = mono[a.min(frames - 1)] as f32;
        let s1 = mono[(a + 1).min(frames - 1)] as f32;
        out.push((s0 + (s1 - s0) * t).round().clamp(-32768.0, 32767.0) as i16);
    }
    Some(out)
}

/// Lazily decoded retail music.  `root` is the materialised `music/` folder.
pub struct MusicBank {
    root: std::path::PathBuf,
    cache: std::cell::RefCell<HashMap<String, Option<std::sync::Arc<Vec<i16>>>>>,
}

impl MusicBank {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            cache: std::cell::RefCell::new(HashMap::new()),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn track(&self, name: &str) -> Option<std::sync::Arc<Vec<i16>>> {
        if let Some(hit) = self.cache.borrow().get(name) {
            return hit.clone();
        }
        let path = self.root.join(format!("{name}.ogg"));
        let decoded = std::fs::read(path)
            .ok()
            .and_then(|bytes| decode_ogg(&bytes))
            .filter(|samples| !samples.is_empty())
            .map(std::sync::Arc::new);
        self.cache
            .borrow_mut()
            .insert(name.to_string(), decoded.clone());
        decoded
    }

    pub fn has(&self, name: &str) -> bool {
        self.root.join(format!("{name}.ogg")).is_file()
    }

    pub fn names(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut out: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                (p.extension().and_then(|s| s.to_str()) == Some("ogg"))
                    .then(|| p.file_stem()?.to_str().map(str::to_string))?
            })
            .collect();
        out.sort();
        out
    }
}

enum AudioMessage {
    Effect(Vec<i16>),
    Music(std::sync::Arc<Vec<i16>>),
    StopMusic,
    /// Channel gains from the Options Sound tab (`GameOptions::mixer_gains`).
    Volume {
        music: f32,
        effects: f32,
    },
    /// "Sound Plays While In Background": false pauses the mixer.
    Background(bool),
}

/// Handle used by the client.  `play` is cheap and never blocks the game
/// thread; mixing happens on the audio thread.
pub struct AudioPlayer {
    tx: Option<Sender<AudioMessage>>,
}

impl AudioPlayer {
    pub fn new(enabled: bool) -> Self {
        if !enabled {
            return Self { tx: None };
        }
        let (tx, rx) = mpsc::channel::<AudioMessage>();
        match std::thread::Builder::new()
            .name("galaxycraft-audio".into())
            .spawn(move || mixer_thread(rx))
        {
            Ok(_) => Self { tx: Some(tx) },
            Err(_) => Self { tx: None },
        }
    }

    pub fn enabled(&self) -> bool {
        self.tx.is_some()
    }

    pub fn play(&self, samples: Option<&[i16]>) {
        if let (Some(tx), Some(samples)) = (&self.tx, samples) {
            let _ = tx.send(AudioMessage::Effect(samples.to_vec()));
        }
    }

    pub fn play_named(&self, bank: &SoundBank, name: &str) {
        self.play(bank.get(name));
    }

    pub fn play_music(&self, track: Option<std::sync::Arc<Vec<i16>>>) {
        if let (Some(tx), Some(track)) = (&self.tx, track) {
            let _ = tx.send(AudioMessage::Music(track));
        }
    }

    pub fn stop_music(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AudioMessage::StopMusic);
        }
    }

    /// Pause/resume all output for the Sound tab's background checkbox.
    pub fn set_background(&self, allowed: bool) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AudioMessage::Background(allowed));
        }
    }

    /// Set the music and effect gains (1.0 = the sample as decoded).
    pub fn set_volumes(&self, music: f32, effects: f32) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AudioMessage::Volume { music, effects });
        }
    }
}

/// A PCM output: blocking writes of mono `i16` blocks at [`SAMPLE_RATE`].
/// Every backend is loaded dynamically, so the binary starts (silent) on a
/// system without it, and tests never need an audio device.
trait PcmOut {
    /// Queue one block; `false` = the device failed and the mixer stops.
    fn write(&mut self, block: &[i16]) -> bool;
}

/// The platform's output: ALSA on Linux/BSD, WinMM on Windows, CoreAudio
/// (AudioQueue) on macOS.
fn open_output() -> Option<Box<dyn PcmOut>> {
    if cfg!(windows) {
        Some(Box::new(winmm::WaveOut::open()?))
    } else if cfg!(target_os = "macos") {
        Some(Box::new(coreaudio::Queue::open()?))
    } else {
        Some(Box::new(alsa::Pcm::open()?))
    }
}

/// ALSA (`libasound.so.2`).
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
mod alsa {
    use std::ffi::c_void;

    type PcmOpen = unsafe extern "C" fn(*mut *mut c_void, *const std::ffi::c_char, i32, i32) -> i32;
    type PcmSetParams = unsafe extern "C" fn(*mut c_void, i32, i32, u32, u32, i32, u32) -> i32;
    type PcmWritei = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> isize;
    type PcmRecover = unsafe extern "C" fn(*mut c_void, i32, i32) -> i32;
    type PcmDrain = unsafe extern "C" fn(*mut c_void) -> i32;
    type PcmClose = unsafe extern "C" fn(*mut c_void) -> i32;

    pub struct Pcm {
        _lib: libloading::Library,
        pcm: *mut c_void,
        writei: PcmWritei,
        recover: PcmRecover,
        drain: PcmDrain,
        close: PcmClose,
    }

    impl Pcm {
        pub fn open() -> Option<Self> {
            unsafe {
                let lib = libloading::Library::new("libasound.so.2").ok()?;
                let open = *lib.get::<PcmOpen>(b"snd_pcm_open\0").ok()?;
                let set_params = *lib.get::<PcmSetParams>(b"snd_pcm_set_params\0").ok()?;
                let writei = *lib.get::<PcmWritei>(b"snd_pcm_writei\0").ok()?;
                let recover = *lib.get::<PcmRecover>(b"snd_pcm_recover\0").ok()?;
                let drain = *lib.get::<PcmDrain>(b"snd_pcm_drain\0").ok()?;
                let close = *lib.get::<PcmClose>(b"snd_pcm_close\0").ok()?;
                let mut pcm: *mut c_void = std::ptr::null_mut();
                if open(&mut pcm, c"default".as_ptr(), 0, 0) != 0 || pcm.is_null() {
                    return None;
                }
                // S16_LE = 2, RW_INTERLEAVED = 3, mono, 200 ms latency.
                if set_params(pcm, 2, 3, 1, super::SAMPLE_RATE, 1, 200_000) != 0 {
                    close(pcm);
                    return None;
                }
                Some(Self {
                    _lib: lib,
                    pcm,
                    writei,
                    recover,
                    drain,
                    close,
                })
            }
        }
    }

    impl super::PcmOut for Pcm {
        fn write(&mut self, block: &[i16]) -> bool {
            let mut written = 0usize;
            while written < block.len() {
                let rc = unsafe {
                    (self.writei)(
                        self.pcm,
                        block[written..].as_ptr() as *const c_void,
                        block.len() - written,
                    )
                };
                if rc < 0 {
                    if unsafe { (self.recover)(self.pcm, rc as i32, 1) } < 0 {
                        return false;
                    }
                } else if rc == 0 {
                    break;
                } else {
                    written += rc as usize;
                }
            }
            true
        }
    }

    impl Drop for Pcm {
        fn drop(&mut self) {
            unsafe {
                (self.drain)(self.pcm);
                (self.close)(self.pcm);
            }
        }
    }
}

/// Windows waveOut (`winmm.dll`): a ring of headers, each written when the
/// device has finished with it (`WHDR_DONE`).
#[cfg_attr(not(windows), allow(dead_code))]
mod winmm {
    use std::ffi::c_void;

    #[repr(C)]
    struct WaveFormatEx {
        format_tag: u16,
        channels: u16,
        samples_per_sec: u32,
        avg_bytes_per_sec: u32,
        block_align: u16,
        bits_per_sample: u16,
        cb_size: u16,
    }

    #[repr(C)]
    pub struct WaveHdr {
        data: *mut i8,
        buffer_length: u32,
        bytes_recorded: u32,
        user: usize,
        flags: u32,
        loops: u32,
        next: *mut WaveHdr,
        reserved: usize,
    }

    const WAVE_MAPPER: u32 = u32::MAX;
    const WHDR_DONE: u32 = 1;
    const WHDR_PREPARED: u32 = 2;
    const BUFFERS: usize = 4;

    type Open = unsafe extern "system" fn(
        *mut *mut c_void,
        u32,
        *const WaveFormatEx,
        usize,
        usize,
        u32,
    ) -> u32;
    type Hdr = unsafe extern "system" fn(*mut c_void, *mut WaveHdr, u32) -> u32;
    type Handle = unsafe extern "system" fn(*mut c_void) -> u32;

    pub struct WaveOut {
        _lib: libloading::Library,
        dev: *mut c_void,
        prepare: Hdr,
        unprepare: Hdr,
        write: Hdr,
        reset: Handle,
        close: Handle,
        /// Created once and never resized: the device holds pointers to
        /// these headers while it plays them.
        headers: Vec<WaveHdr>,
        data: Vec<Vec<i16>>,
        next: usize,
    }

    impl WaveOut {
        pub fn open() -> Option<Self> {
            unsafe {
                let lib = libloading::Library::new("winmm.dll").ok()?;
                let open = *lib.get::<Open>(b"waveOutOpen\0").ok()?;
                let prepare = *lib.get::<Hdr>(b"waveOutPrepareHeader\0").ok()?;
                let unprepare = *lib.get::<Hdr>(b"waveOutUnprepareHeader\0").ok()?;
                let write = *lib.get::<Hdr>(b"waveOutWrite\0").ok()?;
                let reset = *lib.get::<Handle>(b"waveOutReset\0").ok()?;
                let close = *lib.get::<Handle>(b"waveOutClose\0").ok()?;
                let fmt = WaveFormatEx {
                    format_tag: 1, // WAVE_FORMAT_PCM
                    channels: 1,
                    samples_per_sec: super::SAMPLE_RATE,
                    avg_bytes_per_sec: super::SAMPLE_RATE * 2,
                    block_align: 2,
                    bits_per_sample: 16,
                    cb_size: 0,
                };
                let mut dev: *mut c_void = std::ptr::null_mut();
                // CALLBACK_NULL: the mixer polls WHDR_DONE.
                if open(&mut dev, WAVE_MAPPER, &fmt, 0, 0, 0) != 0 || dev.is_null() {
                    return None;
                }
                let headers = (0..BUFFERS)
                    .map(|_| WaveHdr {
                        data: std::ptr::null_mut(),
                        buffer_length: 0,
                        bytes_recorded: 0,
                        user: 0,
                        flags: WHDR_DONE,
                        loops: 0,
                        next: std::ptr::null_mut(),
                        reserved: 0,
                    })
                    .collect();
                Some(Self {
                    _lib: lib,
                    dev,
                    prepare,
                    unprepare,
                    write,
                    reset,
                    close,
                    headers,
                    data: vec![Vec::new(); BUFFERS],
                    next: 0,
                })
            }
        }
    }

    impl super::PcmOut for WaveOut {
        fn write(&mut self, block: &[i16]) -> bool {
            let i = self.next;
            self.next = (i + 1) % BUFFERS;
            let size = std::mem::size_of::<WaveHdr>() as u32;
            // Wait until the device has played this slot (the ring is the
            // blocking point, like ALSA's writei).
            let hdr: *mut WaveHdr = &mut self.headers[i];
            while unsafe { std::ptr::read_volatile(&(*hdr).flags) } & WHDR_DONE == 0 {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            unsafe {
                if (*hdr).flags & WHDR_PREPARED != 0 {
                    (self.unprepare)(self.dev, hdr, size);
                }
                self.data[i].clear();
                self.data[i].extend_from_slice(block);
                (*hdr).data = self.data[i].as_mut_ptr() as *mut i8;
                (*hdr).buffer_length = (block.len() * 2) as u32;
                (*hdr).flags = 0;
                (self.prepare)(self.dev, hdr, size) == 0 && (self.write)(self.dev, hdr, size) == 0
            }
        }
    }

    impl Drop for WaveOut {
        fn drop(&mut self) {
            let size = std::mem::size_of::<WaveHdr>() as u32;
            unsafe {
                (self.reset)(self.dev);
                for h in self.headers.iter_mut() {
                    if h.flags & WHDR_PREPARED != 0 {
                        (self.unprepare)(self.dev, h, size);
                    }
                }
                (self.close)(self.dev);
            }
        }
    }
}

/// macOS CoreAudio (`AudioToolbox` AudioQueue): buffers come back through
/// the queue's callback into a free list the mixer waits on.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod coreaudio {
    use std::ffi::c_void;
    use std::sync::{Arc, Mutex};

    #[repr(C)]
    struct StreamDescription {
        sample_rate: f64,
        format_id: u32,
        format_flags: u32,
        bytes_per_packet: u32,
        frames_per_packet: u32,
        bytes_per_frame: u32,
        channels_per_frame: u32,
        bits_per_channel: u32,
        reserved: u32,
    }

    #[repr(C)]
    pub struct QueueBuffer {
        capacity: u32,
        data: *mut c_void,
        byte_size: u32,
        user: *mut c_void,
        packet_capacity: u32,
        packets: *mut c_void,
        packet_count: u32,
    }

    type Callback = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut QueueBuffer);
    type NewOutput = unsafe extern "C" fn(
        *const StreamDescription,
        Callback,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        u32,
        *mut *mut c_void,
    ) -> i32;
    type Allocate = unsafe extern "C" fn(*mut c_void, u32, *mut *mut QueueBuffer) -> i32;
    type Enqueue = unsafe extern "C" fn(*mut c_void, *mut QueueBuffer, u32, *const c_void) -> i32;
    type Start = unsafe extern "C" fn(*mut c_void, *const c_void) -> i32;
    type Dispose = unsafe extern "C" fn(*mut c_void, u8) -> i32;

    const LIB: &str = "/System/Library/Frameworks/AudioToolbox.framework/AudioToolbox";
    const LINEAR_PCM: u32 = u32::from_be_bytes(*b"lpcm");
    /// Signed integer | packed.
    const FLAGS: u32 = 4 | 8;
    const BUFFERS: usize = 4;
    const BLOCK_BYTES: u32 = 4096;

    /// Free buffers handed back by the queue thread.
    type FreeList = Mutex<Vec<usize>>;

    unsafe extern "C" fn done(user: *mut c_void, _queue: *mut c_void, buf: *mut QueueBuffer) {
        unsafe {
            let free = &*(user as *const FreeList);
            if let Ok(mut v) = free.lock() {
                v.push(buf as usize);
            }
        }
    }

    pub struct Queue {
        _lib: libloading::Library,
        queue: *mut c_void,
        enqueue: Enqueue,
        start: Start,
        dispose: Dispose,
        free: Arc<FreeList>,
        started: bool,
    }

    impl Queue {
        pub fn open() -> Option<Self> {
            unsafe {
                let lib = libloading::Library::new(LIB).ok()?;
                let new_output = *lib.get::<NewOutput>(b"AudioQueueNewOutput\0").ok()?;
                let allocate = *lib.get::<Allocate>(b"AudioQueueAllocateBuffer\0").ok()?;
                let enqueue = *lib.get::<Enqueue>(b"AudioQueueEnqueueBuffer\0").ok()?;
                let start = *lib.get::<Start>(b"AudioQueueStart\0").ok()?;
                let dispose = *lib.get::<Dispose>(b"AudioQueueDispose\0").ok()?;
                let fmt = StreamDescription {
                    sample_rate: super::SAMPLE_RATE as f64,
                    format_id: LINEAR_PCM,
                    format_flags: FLAGS,
                    bytes_per_packet: 2,
                    frames_per_packet: 1,
                    bytes_per_frame: 2,
                    channels_per_frame: 1,
                    bits_per_channel: 16,
                    reserved: 0,
                };
                let free: Arc<FreeList> = Arc::new(Mutex::new(Vec::new()));
                let mut queue: *mut c_void = std::ptr::null_mut();
                // No run loop: callbacks arrive on the queue's own thread.
                let user = Arc::as_ptr(&free) as *mut c_void;
                let null = std::ptr::null_mut();
                if new_output(&fmt, done, user, null, null, 0, &mut queue) != 0 || queue.is_null() {
                    return None;
                }
                for _ in 0..BUFFERS {
                    let mut buf: *mut QueueBuffer = std::ptr::null_mut();
                    if allocate(queue, BLOCK_BYTES, &mut buf) != 0 || buf.is_null() {
                        dispose(queue, 1);
                        return None;
                    }
                    free.lock().ok()?.push(buf as usize);
                }
                Some(Self {
                    _lib: lib,
                    queue,
                    enqueue,
                    start,
                    dispose,
                    free,
                    started: false,
                })
            }
        }
    }

    impl super::PcmOut for Queue {
        fn write(&mut self, block: &[i16]) -> bool {
            let buf = loop {
                if let Some(b) = self.free.lock().ok().and_then(|mut v| v.pop()) {
                    break b as *mut QueueBuffer;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            };
            unsafe {
                let bytes = (block.len() * 2).min((*buf).capacity as usize);
                std::ptr::copy_nonoverlapping(
                    block.as_ptr() as *const u8,
                    (*buf).data as *mut u8,
                    bytes,
                );
                (*buf).byte_size = bytes as u32;
                if (self.enqueue)(self.queue, buf, 0, std::ptr::null()) != 0 {
                    return false;
                }
                if !self.started {
                    self.started = (self.start)(self.queue, std::ptr::null()) == 0;
                }
            }
            true
        }
    }

    impl Drop for Queue {
        fn drop(&mut self) {
            // Immediate dispose stops the callbacks before `free` is dropped.
            unsafe {
                (self.dispose)(self.queue, 1);
            }
        }
    }
}

/// Mix every queued effect and the looping music into one mono stream.
fn mixer_thread(rx: Receiver<AudioMessage>) {
    let Some(mut out) = open_output() else {
        // Keep draining so `play` never fails; the game just runs silent.
        while rx.recv().is_ok() {}
        return;
    };
    let mut mixer = Mixer::default();
    let mut buf = vec![0i16; 1024];
    loop {
        // Drain every queued message before mixing the next block.
        loop {
            match rx.try_recv() {
                Ok(m) => mixer.apply(m),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }
        if mixer.is_idle() {
            match rx.recv() {
                Ok(m) => mixer.apply(m),
                Err(_) => return,
            }
            continue;
        }
        mixer.mix(&mut buf);
        if !out.write(&buf) {
            return;
        }
    }
}

/// Software mixer state: up to 8 effects plus one looping music track.
struct Mixer {
    active: Vec<(Vec<i16>, usize)>,
    music: Option<(std::sync::Arc<Vec<i16>>, usize)>,
    music_gain: f32,
    effect_gain: f32,
    /// False while the client is unfocused and the background checkbox is off.
    background: bool,
}

impl Default for Mixer {
    /// Music at 3/5 so effects remain audible, effects as decoded.
    fn default() -> Self {
        Self {
            active: Vec::new(),
            music: None,
            music_gain: 0.6,
            effect_gain: 1.0,
            background: true,
        }
    }
}

impl Mixer {
    fn apply(&mut self, m: AudioMessage) {
        match m {
            AudioMessage::Effect(samples) => {
                if !samples.is_empty() && self.active.len() < 8 {
                    self.active.push((samples, 0));
                }
            }
            AudioMessage::Music(track) => {
                self.music = (!track.is_empty()).then_some((track, 0));
            }
            AudioMessage::StopMusic => self.music = None,
            AudioMessage::Volume { music, effects } => {
                // Unity is the loudest: a channel is never amplified past
                // its recording (no clipping from the volume alone).
                self.music_gain = music.clamp(0.0, 1.0);
                self.effect_gain = effects.clamp(0.0, 1.0);
            }
            AudioMessage::Background(allowed) => self.background = allowed,
        }
    }

    fn is_idle(&self) -> bool {
        !self.background || (self.active.is_empty() && self.music.is_none())
    }

    fn mix(&mut self, buf: &mut [i16]) {
        buf.fill(0);
        if !self.background {
            return;
        }
        for (samples, pos) in self.active.iter_mut() {
            for (i, dst) in buf.iter_mut().enumerate() {
                if *pos + i < samples.len() {
                    let s = (samples[*pos + i] as f32 * self.effect_gain).round() as i32;
                    *dst = (*dst as i32 + s).clamp(-32768, 32767) as i16;
                }
            }
            *pos += buf.len();
        }
        self.active.retain(|(samples, pos)| *pos < samples.len());
        // Music is looped at its own gain (3/5 by default).
        if let Some((samples, pos)) = self.music.as_mut() {
            for (i, dst) in buf.iter_mut().enumerate() {
                let src = (*pos + i) % samples.len();
                let mixed = *dst as i32 + (samples[src] as f32 * self.music_gain).round() as i32;
                *dst = mixed.clamp(-32768, 32767) as i16;
            }
            *pos = (*pos + buf.len()) % samples.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_switch_pauses_without_advancing_the_mix() {
        let mut m = Mixer::default();
        m.apply(AudioMessage::Music(std::sync::Arc::new(vec![10, -10])));
        let mut buf = [0i16; 2];
        m.mix(&mut buf);
        assert_eq!(buf, [6, -6]);
        m.apply(AudioMessage::Background(false));
        assert!(m.is_idle(), "a paused mixer waits for messages");
        let mut silent = [9i16; 2];
        m.mix(&mut silent);
        assert_eq!(silent, [0, 0]);
        m.apply(AudioMessage::Background(true));
        let mut resumed = [0i16; 2];
        m.mix(&mut resumed);
        assert_eq!(resumed, [6, -6], "resumes at the same music position");
    }

    #[test]
    fn mixer_sums_effects_clamps_and_loops_music_at_three_fifths() {
        let mut m = Mixer::default();
        assert!(m.is_idle());
        m.apply(AudioMessage::Effect(vec![30_000, 100, 7]));
        m.apply(AudioMessage::Effect(vec![30_000, -50]));
        m.apply(AudioMessage::Music(std::sync::Arc::new(vec![10, -10])));
        let mut buf = [0i16; 4];
        m.mix(&mut buf);
        // 30000 + 30000 clamps; music adds 10 * 3 / 5 = 6 and -6.
        assert_eq!(buf, [32_767, 44, 13, -6]);
        // Effects shorter than the block are done; the music keeps looping.
        m.mix(&mut buf);
        assert_eq!(buf, [6, -6, 6, -6]);
        m.apply(AudioMessage::StopMusic);
        assert!(m.is_idle());
        // At most eight overlapping effects.
        for _ in 0..10 {
            m.apply(AudioMessage::Effect(vec![1; 8]));
        }
        m.mix(&mut buf);
        assert_eq!(buf, [8; 4]);
    }

    #[test]
    fn the_volume_message_scales_music_and_effects() {
        let mut m = Mixer::default();
        m.apply(AudioMessage::Volume {
            music: 0.0,
            effects: 0.5,
        });
        m.apply(AudioMessage::Effect(vec![1000, -1000]));
        m.apply(AudioMessage::Music(std::sync::Arc::new(vec![10_000])));
        let mut buf = [0i16; 2];
        m.mix(&mut buf);
        assert_eq!(buf, [500, -500], "music muted, effects halved");
        m.apply(AudioMessage::Volume {
            music: 1.2,
            effects: 0.0,
        });
        m.apply(AudioMessage::Effect(vec![1000, 1000]));
        m.mix(&mut buf);
        assert_eq!(buf, [10_000, 10_000], "capped at 100 %, never boosted");
    }

    #[test]
    fn a_tiny_mono_wav_decodes() {
        // 4 frames, 16-bit mono, 44.1 kHz.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36u32 + 8).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&88_200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&8u32.to_le_bytes());
        for s in [0i16, 1000, -1000, 32767] {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        let wav = Wav::parse(&bytes).expect("valid wav");
        assert_eq!(wav.rate, 44_100);
        assert_eq!(wav.channels, 1);
        assert_eq!(wav.samples, vec![0, 1000, -1000, 32767]);
        assert_eq!(wav.to_mono_44k(), wav.samples);
        // Map sounds go through the same path; other bytes are refused.
        assert_eq!(decode_sound(&bytes), Some(wav.samples.clone()));
        assert_eq!(decode_sound(b"OggS-but-not-really"), None);
        assert_eq!(decode_sound(b"garbage"), None);
    }

    #[test]
    fn vorbis_music_decodes_to_the_mixer_rate() {
        let bytes = include_bytes!("../fixtures/audio/tone.ogg");
        let samples = decode_ogg(bytes).expect("vorbis decodes");
        // One second at 44.1 kHz, within the decoder's padding tolerance.
        assert!(
            (40_000..=48_000).contains(&samples.len()),
            "{} samples",
            samples.len()
        );
        assert!(samples.iter().any(|s| s.abs() > 1000), "silent decode");
    }

    #[test]
    fn music_bank_decodes_lazily_and_caches() {
        let dir = std::env::temp_dir().join(format!("gc-music-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tone.ogg"),
            include_bytes!("../fixtures/audio/tone.ogg"),
        )
        .unwrap();
        let bank = MusicBank::new(&dir);
        assert!(bank.has("tone"));
        assert_eq!(bank.names(), vec!["tone".to_string()]);
        let first = bank.track("tone").expect("track");
        let second = bank.track("tone").expect("cached track");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(!first.is_empty());
        assert!(bank.track("missing").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}

// ===========================================================================
// Merged from src/wave14/agent06.rs
// ===========================================================================
// Wave 14 translation batch 06 (staging).
//
// Window: `0067a000..=0067bfff`.  The window is the mixer core of the
// retail sound system: the per-thread sound object that owns 128
// DirectSound voices, the voice-handle generation scheme, the small
// `Master`/`Music`/`Sfx` group buffers, and the "WolfeLimiter" dynamics
// processor that scales mixed sample blocks.
//
// ## Recovered layout
//
// The sound object (returned by the per-thread getter `FUN_006289e0`) is:
//
// | offset | shape | meaning |
// |---|---|---|
// | `+0x00` | `u32` | DirectSound device interface |
// | `+0x04` | `u32` | `Master` group buffer |
// | `+0x08` | `u32` | `Music` group buffer |
// | `+0x0c` | `u32` | `Sfx` group buffer |
// | `+0x10` | 128 x `0x18` | voice table, indexed by the low 16 bits of a handle |
//
// One voice record is:
//
// | offset | shape | meaning |
// |---|---|---|
// | `+0x00` | `u32` | DirectSound buffer interface |
// | `+0x04` | `u16` | generation, part of the packed handle |
// | `+0x06` | `u8` | query/play state byte (non-zero suppresses "finished") |
// | `+0x07` | `u8` | in-use marker scanned by the allocator |
// | `+0x08` | `u32` | pending sound descriptor token when buffer creation is deferred |
// | `+0x0c` | `u32` | pending handle, `0xffff` when unused |
// | `+0x10` | `f32` | volume |
// | `+0x14` | `f32` | pan |
//
// A handle is `(generation << 16) | index`; generation `0` is never valid.
// Releasing a voice advances the generation (`0` becomes `1`, `0xffff`
// wraps to `0`) so stale handles no longer match.
//
// ## Limiter layout
//
// The "WolfeLimiter" object stores its running envelope state at `+0x04`
// (consumed by `FUN_00679a40`, outside this window) and its configuration
// and derived gains at `+0x44..=0x88`.  Values are decibels converted with
// `10^(db/20)` and one-pole coefficients `exp(-2 / (ms * rate * 0.001))`.
// The field-by-field table is on [`W14a06Limiter`].
//
// ## Translation notes
//
// * Backend calls into the DirectSound wrappers (`FUN_00984xxx`,
//   `FUN_00985xxx`) are abstracted exactly as in the wave 13 batches:
//   query results enter as [`W14a06BackendQuery`], create calls as
//   [`W14a06PlayProbe`], and release helpers return the handles the caller
//   still has to release.  No platform call is executed here.
// * The release rows return the released buffer handle (or the whole
//   matching record) instead of the retail `bool`; `Some`/a non-zero value
//   is the retail `true`.
// * `FUN_0067bd50` indexes the three group buffers without a bounds check
//   in retail; the translation reports `None` for an out-of-range group.
//
// Skipped rows, and why:
//
// * `FUN_0067a020`, `FUN_0067a110`, `FUN_0067a1d0`: C++ initialiser /
//   destructor bodies whose tails build a `0x84`-byte exception object and
//   end in the non-returning throw helper.  Their resource-release prefix
//   is the same sequence already translated for `FUN_0067bb30`.
// * `FUN_0067a164`, `FUN_0067b405`: mid-instruction fragments reading
//   `unaff_EBP`/`unaff_ESI`/`unaff_EDI`; `FUN_0067b405` begins with the
//   byte pair `e6 ff` (`out 0xff, al`) and overlaps `FUN_0067b3c0`.
// * `FUN_0067a410`: scalar deleting destructor; a base-destructor call and
//   an operator delete with no arithmetic or state to reproduce.
// * `FUN_0067a440`, `FUN_0067bff0`: SEH frame constructors that copy
//   `std::string` temporaries and install vtables from opaque descriptors.
// * `FUN_0067a705`, `FUN_0067a79b`: six-byte catch-all stubs returning the
//   address of a handler label, not functions.
// * `FUN_0067a810`, `FUN_0067b860`: SEH frame prologues that end in the
//   non-returning frame installer.
// * `FUN_0067aa00`: device construction with eight opaque wrapper calls,
//   a copied `WAVEFORMAT` block at `d3e620`, and a non-returning tail; the
//   visible layout it writes is already covered by the channel helpers.
// * `FUN_0067b060`: notification callback whose only effect is a device
//   query and a conditional seek through opaque wrappers; every path
//   returns `0`.
//
// Nothing here is wired into `World`, `SimUnit`, the renderer or the
// client, and no save/replay state is introduced.

// The playback rows mirror their cdecl signatures (descriptor, volume, pan,
// allocated handle, backend results); splitting them into structs would hide
// the recovered argument order.
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of voice records in the sound object (retail `0x80`).
pub const W14A06_MAX_CHANNELS: usize = 0x80;
/// Byte stride of one voice record (retail `0x18`).
pub const W14A06_CHANNEL_STRIDE: usize = 0x18;
/// Offset of voice slot `0` inside the sound object.
pub const W14A06_CHANNEL_TABLE_OFFSET: usize = 0x10;
/// Handle value written when allocation or lookup fails.
pub const W14A06_INVALID_HANDLE: u32 = 0xffff;
/// Backend codes treated as "the buffer handle is no longer alive"
/// (`0x24` and `0x0b` in the three voice query rows).
pub const W14A06_DEAD_BUFFER_CODES: [u32; 2] = [0x24, 0x0b];
/// `FUN_00984eb0` result that defers buffer creation.
pub const W14A06_DEFERRED_CREATE_STATUS: u32 = 0x36;
/// Number of group buffers (`Master`, `Music`, `Sfx`).
pub const W14A06_GROUP_COUNT: usize = 3;

// ---------------------------------------------------------------------------
// Recovered records
// ---------------------------------------------------------------------------

/// One 24-byte voice record of the sound object.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct W14a06Channel {
    /// `+0x00`: DirectSound buffer interface.
    pub buffer: u32,
    /// `+0x04`: handle generation.
    pub generation: u16,
    /// `+0x06`: query/play state byte.
    pub flags_06: u8,
    /// `+0x07`: in-use marker used by the allocator.
    pub valid: bool,
    /// `+0x08`: pending sound descriptor token (deferred creation).
    pub data: u32,
    /// `+0x0c`: pending packed handle.
    pub pending: u32,
    /// `+0x10`: volume.
    pub volume: f32,
    /// `+0x14`: pan.
    pub pan: f32,
}

impl W14a06Channel {
    /// The record as `FUN_00679c50` initialises it: blank buffer, generation
    /// `0`, both flag bytes clear, and the pending field set to `0xffff`.
    pub fn cleared() -> Self {
        Self {
            pending: W14A06_INVALID_HANDLE,
            ..Self::default()
        }
    }
}

/// The three group buffers plus the device, at `+0x00..=0x0c` of the sound
/// object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06Sound {
    /// `+0x00`: DirectSound device.
    pub device: u32,
    /// `+0x04`, `+0x08`, `+0x0c`: `Master`, `Music`, `Sfx`.
    pub groups: [u32; W14A06_GROUP_COUNT],
}

/// Result of a buffer query.
///
/// The retail rows call `FUN_00985020`, `FUN_00984fe0` or `FUN_009850c0`
/// wrappers; a return code of `0x24` or `0x0b` means the wrapper could not
/// resolve the buffer object and the voice is recycled.  Any other code
/// leaves a value (a status byte or a dword) to interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a06BackendQuery<T> {
    /// The call succeeded and produced this value.
    Value(T),
    /// The buffer object was already gone.
    Dead,
}

impl<T> W14a06BackendQuery<T> {
    /// Classify a raw wrapper result using the shared `0x24`/`0x0b` test.
    pub fn from_status(code: u32, value: T) -> Self {
        if code == W14A06_DEAD_BUFFER_CODES[0] || code == W14A06_DEAD_BUFFER_CODES[1] {
            Self::Dead
        } else {
            Self::Value(value)
        }
    }
}

/// A voice record that was matched by a packed handle and released.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a06ChannelRelease {
    /// Index of the matched voice in the table.
    pub index: usize,
    /// Buffer handle that the caller must release (may be `0`).
    pub buffer: u32,
}

/// One decoded sound descriptor, as used by `FUN_0067b0a0` and
/// `FUN_0067bc60` (fields `+0x00`, `+0x04`, `+0x08`, `+0x0c`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06SoundData {
    /// `+0x00`: source/parser object.
    pub source: u32,
    /// `+0x04`: prepared state (zero until `FUN_0067bc60` succeeds).
    pub prepared: u32,
    /// `+0x08`: decoded sample buffer.
    pub samples: u32,
    /// `+0x0c`: group index (`0` Master, `1` Music, `2` Sfx).
    pub group: u32,
}

/// Backend results consumed by `FUN_0067bc60`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06PrepareProbe {
    /// Result of `FUN_00984a10(source, &parsed, 0, 0, 0)`.
    pub parse_status: u32,
    /// The `parsed` value that call wrote.
    pub parsed: u32,
    /// Result of `FUN_009849c0(source, &allocated, 1)`.
    pub alloc_status: u32,
    /// The `allocated` value that call wrote.
    pub allocated: u32,
}

/// Backend results consumed by `FUN_0067b0a0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06PlayProbe {
    /// Results of the `FUN_0067bc60` preparation step.
    pub prepare: W14a06PrepareProbe,
    /// Result of `FUN_00984eb0(device, -1, source, loop, voice)`.
    pub create_status: u32,
    /// The buffer interface that call stored in the voice record.
    pub created_buffer: u32,
}

/// Outcome of the three "start a voice" rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14a06PlayResult {
    /// The sound object has no device yet (`*this == 0`).
    NoDevice,
    /// Bad index/generation, or a create call failed.
    InvalidHandle,
    /// Creation returned a hard error while a buffer already existed.
    Failed,
    /// Parameters were stored and buffer creation is deferred.
    Deferred {
        /// Matched voice slot.
        index: usize,
    },
    /// The voice was filled in; the caller still issues the group, volume,
    /// pan and `FUN_0067bbf0(0)` calls described by the record.
    Started {
        /// Matched voice slot.
        index: usize,
        /// Group buffer the retail body selects (`sound.groups[data.group]`).
        group: u32,
    },
}

/// Descriptor used by `FUN_0067b240`: a one-shot sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06Sample {
    /// `+0x00`: sample data handed to `FUN_00984eb0`.
    pub samples: u32,
    /// `+0x04`: group index.
    pub group: u32,
    /// `+0x08`: packed handle written by the row.
    pub handle: u32,
}

/// Descriptor used by `FUN_0067b300`: a streamed sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06StreamData {
    /// `+0x00`: reserved source word.
    pub field_00: u32,
    /// `+0x04`: reserved second word.
    pub field_04: u32,
    /// `+0x08`: sample data handed to `FUN_00984eb0`.
    pub samples: u32,
    /// `+0x0c`: group index.
    pub group: u32,
    /// `+0x10`: packed handle written by the row.
    pub handle: u32,
}

/// The record released by `FUN_0067bb00`: `{ buffer, ?, handle }`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06Voice {
    /// `+0x00`: buffer interface.
    pub buffer: u32,
    /// `+0x04`: unused by the row.
    pub reserved: u32,
    /// `+0x08`: packed handle.
    pub handle: u32,
}

/// Handles reported by `FUN_0067bb00`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a06VoiceRelease {
    /// Matched voice record, if the handle was still live.
    pub handle: Option<W14a06ChannelRelease>,
    /// Buffer interface the caller must release.
    pub buffer: u32,
}

/// The record released by `FUN_0067bb30`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a06Stream {
    /// `+0x00`: buffer interface.
    pub buffer: u32,
    /// `+0x04`: unused by the row.
    pub field_04: u32,
    /// `+0x08`: owned sample memory.
    pub memory: u32,
    /// `+0x0c`: unused by the row.
    pub field_0c: u32,
    /// `+0x10`: packed handle.
    pub handle: u32,
    /// `+0x14`: owned text/data pointer.
    pub text: u32,
    /// `+0x18`: length of the `text` allocation, `0` when absent.
    pub text_len: u32,
}

/// Handles reported by `FUN_0067bb30`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a06StreamRelease {
    /// Matched voice record, if the handle was still live.
    pub handle: Option<W14a06ChannelRelease>,
    /// Text/data block to free, with its length.
    pub text: Option<(u32, u32)>,
    /// Sample memory the caller must release.
    pub buffer: u32,
}

/// Handles reported by `FUN_0067a2e0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a06CloseActions {
    /// `Master`, `Music` and `Sfx` buffers in release order.
    pub groups: [u32; W14A06_GROUP_COUNT],
    /// Limiter object released after the groups (may be `0`).
    pub limiter: u32,
    /// Device released last.
    pub device: u32,
}

// ---------------------------------------------------------------------------
// Voice table
// ---------------------------------------------------------------------------

/// Advance a voice generation.
///
/// Shared by `FUN_0067a340` and `FUN_0067a3c0`: `0` becomes `1`, all other
/// values increment modulo 65536 (`0xffff` wraps to `0`).
pub fn w14a06_next_generation(generation: u16) -> u16 {
    if generation == 0 {
        1
    } else {
        generation.wrapping_add(1)
    }
}

/// `0x0067a3c0`: release the voice record at `channel`.
///
/// The retail body releases `channel->buffer` through `FUN_00985120`, clears
/// the flag word and the pending token, advances the generation and stores
/// `0xffff` in the pending handle.  Volume and pan are deliberately left
/// untouched.  Returns the buffer handle the caller must release.
pub fn w14a06_release_channel(channel: &mut W14a06Channel) -> u32 {
    let buffer = channel.buffer;
    channel.buffer = 0;
    channel.generation = w14a06_next_generation(channel.generation);
    channel.flags_06 = 0;
    channel.valid = false;
    channel.data = 0;
    channel.pending = W14A06_INVALID_HANDLE;
    buffer
}

/// `0x0067a340`: release the voice selected by a packed handle.
///
/// The index is the low 16 bits and the generation the high 16 bits.  An
/// index of `0x80` or more, generation `0`, or a generation mismatch leaves
/// the table untouched and returns `None` (the retail `false`).
pub fn w14a06_release_channel_handle(
    channels: &mut [W14a06Channel],
    handle: u32,
) -> Option<W14a06ChannelRelease> {
    let index = (handle & 0xffff) as usize;
    let generation = (handle >> 16) as u16;
    if index >= W14A06_MAX_CHANNELS || generation == 0 {
        return None;
    }
    let channel = channels.get_mut(index)?;
    if channel.generation != generation {
        return None;
    }
    let buffer = w14a06_release_channel(channel);
    Some(W14a06ChannelRelease { index, buffer })
}

/// `0x0067a9c0`: resolve a packed handle to a voice table index.
///
/// The row returns a pointer to the voice record, or null; the translation
/// returns the index.  Only the index and generation are checked, not the
/// in-use marker.
pub fn w14a06_find_channel(channels: &[W14a06Channel], handle: u32) -> Option<usize> {
    let index = (handle & 0xffff) as usize;
    let generation = (handle >> 16) as u16;
    if index >= W14A06_MAX_CHANNELS || generation == 0 {
        return None;
    }
    let channel = channels.get(index)?;
    if channel.generation == generation {
        Some(index)
    } else {
        None
    }
}

/// `0x0067acc0`: the voice has a deferred sound descriptor.
pub fn w14a06_channel_has_data(channel: &W14a06Channel) -> bool {
    channel.data != 0
}

// ---------------------------------------------------------------------------
// Voice state queries
// ---------------------------------------------------------------------------

/// `0x0067ac20`: has the voice finished?
///
/// A bad handle counts as finished.  A set `+0x06` byte means the voice is
/// still owned by its caller.  With no live buffer the answer is "no
/// deferred descriptor left"; otherwise a dead buffer query recycles the
/// voice and reports finished, and a live status byte is inverted.
pub fn w14a06_channel_is_finished(
    channels: &mut [W14a06Channel],
    handle: u32,
    query: W14a06BackendQuery<bool>,
) -> bool {
    let Some(index) = w14a06_find_channel(channels, handle) else {
        return true;
    };
    let channel = &mut channels[index];
    if channel.flags_06 != 0 {
        return false;
    }
    if channel.buffer == 0 {
        return channel.data == 0;
    }
    match query {
        W14a06BackendQuery::Value(playing) => !playing,
        W14a06BackendQuery::Dead => {
            channel.buffer = 0;
            w14a06_release_channel(channel);
            true
        }
    }
}

/// `0x0067acd0`: is the voice still playing or still pending?
///
/// A bad handle answers `false`.  Without a live buffer the deferred
/// descriptor flag is returned; otherwise a dead buffer query recycles the
/// voice and answers `false`, and a live status byte is returned unchanged.
pub fn w14a06_channel_is_playing(
    channels: &mut [W14a06Channel],
    handle: u32,
    query: W14a06BackendQuery<bool>,
) -> bool {
    let Some(index) = w14a06_find_channel(channels, handle) else {
        return false;
    };
    let channel = &mut channels[index];
    if channel.buffer == 0 {
        return channel.data != 0;
    }
    match query {
        W14a06BackendQuery::Value(playing) => playing,
        W14a06BackendQuery::Dead => {
            channel.buffer = 0;
            w14a06_release_channel(channel);
            false
        }
    }
}

/// `0x0067a7c0`: run a dword query against a voice's buffer.
///
/// With no buffer the row answers `0` without calling the backend.  A dead
/// buffer query clears the buffer field, recycles the voice and answers `0`;
/// any other result is returned as-is.
pub fn w14a06_channel_query(channel: &mut W14a06Channel, query: W14a06BackendQuery<u32>) -> u32 {
    if channel.buffer == 0 {
        return 0;
    }
    match query {
        W14a06BackendQuery::Value(value) => value,
        W14a06BackendQuery::Dead => {
            channel.buffer = 0;
            w14a06_release_channel(channel);
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Voice parameter setters
// ---------------------------------------------------------------------------

/// `0x0067bb80`: set the pan of the voice selected by `handle`.
///
/// A voice with a deferred descriptor stores the value in the record and
/// succeeds without a backend call.  Otherwise the live buffer is updated
/// through `FUN_00985090`; `backend_ok` carries that call's success.  A bad
/// handle or an absent buffer answers `false`.
pub fn w14a06_set_channel_pan(
    channels: &mut [W14a06Channel],
    handle: u32,
    pan: f32,
    backend_ok: bool,
) -> bool {
    let Some(index) = w14a06_find_channel(channels, handle) else {
        return false;
    };
    let channel = &mut channels[index];
    if channel.data != 0 {
        channel.pan = pan;
        true
    } else if channel.buffer != 0 {
        backend_ok
    } else {
        false
    }
}

/// `0x0067bce0`: set the volume of the voice selected by `handle`.
///
/// Mirror of the pan row; the deferred branch writes `+0x10` and the live
/// branch reports `FUN_009850f0(buffer, volume) == 0` through `backend_ok`.
pub fn w14a06_set_channel_volume(
    channels: &mut [W14a06Channel],
    handle: u32,
    volume: f32,
    backend_ok: bool,
) -> bool {
    let Some(index) = w14a06_find_channel(channels, handle) else {
        return false;
    };
    let channel = &mut channels[index];
    if channel.data != 0 {
        channel.volume = volume;
        true
    } else if channel.buffer != 0 {
        backend_ok
    } else {
        false
    }
}

/// `0x0067bd50`: set the volume of one of the three group buffers.
///
/// The retail body indexes `this[group + 1]` without a bounds check; the
/// translation reports `None` when the device is absent or the group index
/// is not `0..=2`.  `Some((buffer, volume))` is the `FUN_009851b0` call the
/// caller still has to make.
pub fn w14a06_set_group_volume(
    device: u32,
    groups: &[u32; W14A06_GROUP_COUNT],
    group: usize,
    volume: f32,
) -> Option<(u32, f32)> {
    if device == 0 || group >= W14A06_GROUP_COUNT {
        return None;
    }
    Some((groups[group], volume))
}

// ---------------------------------------------------------------------------
// Destructor fragments
// ---------------------------------------------------------------------------

/// `0x0067bb00`: release a voice record `{ buffer, ?, handle }`.
///
/// The packed handle is released first, then the buffer interface.  Both
/// handles are reported; the record's buffer field is cleared.
pub fn w14a06_release_voice(
    voice: &mut W14a06Voice,
    channels: &mut [W14a06Channel],
) -> W14a06VoiceRelease {
    let handle = w14a06_release_channel_handle(channels, voice.handle);
    let buffer = voice.buffer;
    voice.buffer = 0;
    W14a06VoiceRelease { handle, buffer }
}

/// `0x0067bb30`: release a stream record `{ buffer, ?, memory, ?, handle, text, len }`.
///
/// Order: release the packed handle; if there is owned sample memory, free
/// the text block when its length is non-zero (the retail body also writes
/// a zero byte through the text pointer), then release and clear the sample
/// memory.  Everything is reported to the caller.
pub fn w14a06_release_stream(
    stream: &mut W14a06Stream,
    channels: &mut [W14a06Channel],
) -> W14a06StreamRelease {
    let handle = w14a06_release_channel_handle(channels, stream.handle);
    let mut text = None;
    let mut buffer = 0;
    if stream.memory != 0 {
        if stream.text_len != 0 {
            text = Some((stream.text, stream.text_len));
            stream.text_len = 0;
        }
        buffer = stream.memory;
        stream.memory = 0;
    }
    W14a06StreamRelease {
        handle,
        text,
        buffer,
    }
}

/// `0x0067a2e0`: close the sound object.
///
/// When a device is present the row releases `Master`, `Music`, `Sfx`, then
/// the global limiter, then the device, clearing every field.  The three
/// group handles, the limiter and the device are returned in release order.
pub fn w14a06_close_sound(
    group_handles: &mut [u32; W14A06_GROUP_COUNT + 1],
    limiter: &mut u32,
) -> Option<W14a06CloseActions> {
    if group_handles[0] == 0 {
        return None;
    }
    let mut groups = [0u32; W14A06_GROUP_COUNT];
    for index in 0..W14A06_GROUP_COUNT {
        groups[index] = group_handles[index + 1];
        group_handles[index + 1] = 0;
    }
    let limiter_handle = *limiter;
    *limiter = 0;
    let device = group_handles[0];
    group_handles[0] = 0;
    Some(W14a06CloseActions {
        groups,
        limiter: limiter_handle,
        device,
    })
}

// ---------------------------------------------------------------------------
// Playback start
// ---------------------------------------------------------------------------

/// `0x0067bbf0`: start (or re-start) the voice at `channel`.
///
/// The retail body queries the buffer status: a dead buffer recycles the
/// voice; a playing buffer is re-started with `FUN_009850c0(buffer, flags)`
/// and the call's success is the result; an idle buffer just clears the
/// `+0x06` byte.  `play_status` is the `FUN_009850c0` result.
pub fn w14a06_channel_play(
    channel: &mut W14a06Channel,
    flags: u32,
    query: W14a06BackendQuery<bool>,
    play_status: u32,
) -> bool {
    let _ = flags;
    if channel.buffer == 0 {
        return false;
    }
    match query {
        W14a06BackendQuery::Dead => {
            channel.buffer = 0;
            w14a06_release_channel(channel);
        }
        W14a06BackendQuery::Value(true) => return play_status == 0,
        W14a06BackendQuery::Value(false) => {}
    }
    channel.flags_06 = 0;
    true
}

/// `0x0067bc60`: ensure a sound descriptor has a decoded buffer.
///
/// The row only parses when `prepared` is zero.  A failing parse returns
/// `false`; success forces group `2` (`Sfx`) and a zero sample pointer,
/// then asks the allocator only when the parser produced no pointer.  The
/// row reports `true` even when the allocator itself fails (the prepared
/// field is then left at zero).
pub fn w14a06_stream_prepare(data: &mut W14a06SoundData, probe: W14a06PrepareProbe) -> bool {
    if data.prepared != 0 {
        return true;
    }
    if probe.parse_status != 0 {
        return false;
    }
    data.group = 2;
    data.samples = 0;
    if probe.parsed == 0 && data.prepared == 0 {
        data.prepared = probe.allocated;
        if probe.alloc_status != 0 {
            data.prepared = 0;
        }
    }
    true
}

/// `0x0067b0a0`: play a decoded descriptor on the voice named by `handle`.
///
/// The generation is validated before anything else.  The descriptor is
/// prepared, the voice's pending fields are reset, and buffer creation is
/// attempted with loop flag `1`.  A `0x36` result (or a missing buffer)
/// stores the descriptor token, volume, handle and pan for a later retry;
/// any other non-zero result fails; success leaves the group/volume/pan
/// calls to the caller.  The row then calls `FUN_0067bbf0(0)` (translated
/// separately), clamps the generation to at least `1` and marks the voice
/// in use.
#[allow(clippy::too_many_arguments)]
pub fn w14a06_play_on_channel(
    sound: &W14a06Sound,
    channels: &mut [W14a06Channel],
    data: &mut W14a06SoundData,
    data_token: u32,
    handle: u32,
    volume: f32,
    pan: f32,
    probe: W14a06PlayProbe,
) -> W14a06PlayResult {
    let index = (handle & 0xffff) as usize;
    let generation = (handle >> 16) as u16;
    if index >= W14A06_MAX_CHANNELS || generation == 0 {
        return W14a06PlayResult::InvalidHandle;
    }
    let Some(channel) = channels.get_mut(index) else {
        return W14a06PlayResult::InvalidHandle;
    };
    if channel.generation != generation {
        return W14a06PlayResult::InvalidHandle;
    }
    let _ = w14a06_stream_prepare(data, probe.prepare);
    channel.data = 0;
    channel.pending = W14A06_INVALID_HANDLE;
    channel.volume = 0.0;
    channel.pan = 0.0;
    channel.valid = false;
    channel.buffer = probe.created_buffer;
    if probe.create_status == W14A06_DEFERRED_CREATE_STATUS || channel.buffer == 0 {
        channel.data = data_token;
        channel.volume = volume;
        channel.pending = handle;
        channel.pan = pan;
    } else if probe.create_status != 0 {
        return W14a06PlayResult::Failed;
    }
    channel.generation = channel.generation.max(1);
    channel.valid = true;
    if channel.data != 0 {
        W14a06PlayResult::Deferred { index }
    } else {
        let group = sound.groups.get(data.group as usize).copied().unwrap_or(0);
        W14a06PlayResult::Started { index, group }
    }
}

/// `0x0067b1e0`: allocate a voice and play a descriptor.
///
/// The retail body returns `0xffff` immediately when the sound object has no
/// device, otherwise it allocates a packed handle with `FUN_00679d40` and
/// forwards to `FUN_0067b0a0`.  The allocated handle is an input here.
#[allow(clippy::too_many_arguments)]
pub fn w14a06_play_sound(
    sound: &W14a06Sound,
    channels: &mut [W14a06Channel],
    data: &mut W14a06SoundData,
    data_token: u32,
    volume: f32,
    pan: f32,
    allocated: u32,
    probe: W14a06PlayProbe,
) -> W14a06PlayResult {
    if sound.device == 0 {
        return W14a06PlayResult::NoDevice;
    }
    w14a06_play_on_channel(
        sound, channels, data, data_token, allocated, volume, pan, probe,
    )
}

/// `0x0067b240`: allocate and start a one-shot sample descriptor.
///
/// The row stores the allocated handle at descriptor `+0x08`, validates it,
/// creates the buffer with loop flag `0`, and on success applies the group,
/// volume and pan calls and marks the voice in use.  Returns the retail
/// `bool`.
#[allow(clippy::too_many_arguments)]
pub fn w14a06_play_sample(
    sound: &W14a06Sound,
    channels: &mut [W14a06Channel],
    sample: &mut W14a06Sample,
    volume: f32,
    pan: f32,
    allocated: u32,
    create_status: u32,
    created_buffer: u32,
) -> bool {
    if sound.device == 0 {
        return false;
    }
    sample.handle = allocated;
    let Some(index) = w14a06_find_channel(channels, allocated) else {
        return false;
    };
    if create_status != 0 {
        return false;
    }
    let channel = &mut channels[index];
    channel.buffer = created_buffer;
    channel.volume = volume;
    channel.pan = pan;
    channel.generation = channel.generation.max(1);
    channel.valid = true;
    true
}

/// `0x0067b300`: allocate and start a streamed sample descriptor.
///
/// Same shape as `FUN_0067b240` but the handle is stored at descriptor
/// `+0x10`, the sample data comes from `+0x08`, the group index from
/// `+0x0c`, and buffer creation uses loop flag `1`.
#[allow(clippy::too_many_arguments)]
pub fn w14a06_play_stream(
    sound: &W14a06Sound,
    channels: &mut [W14a06Channel],
    stream: &mut W14a06StreamData,
    volume: f32,
    pan: f32,
    allocated: u32,
    create_status: u32,
    created_buffer: u32,
) -> bool {
    if sound.device == 0 {
        return false;
    }
    stream.handle = allocated;
    let Some(index) = w14a06_find_channel(channels, allocated) else {
        return false;
    };
    if create_status != 0 {
        return false;
    }
    let channel = &mut channels[index];
    channel.buffer = created_buffer;
    channel.volume = volume;
    channel.pan = pan;
    channel.generation = channel.generation.max(1);
    channel.valid = true;
    true
}

// ---------------------------------------------------------------------------
// Small interface helpers
// ---------------------------------------------------------------------------

/// `0x0067a6a0`: map a virtual `create` result to an interface status.
///
/// The retail body calls `(*(*this + 4))(arg2, arg3)`, stores the result in
/// `*param_4` and returns `0x16` when it is non-zero, `0` otherwise.  The
/// callee's result is the input here; the tuple is `(status, stored value)`.
pub fn w14a06_map_create_result(created: u32) -> (u32, u32) {
    (if created != 0 { 0x16 } else { 0 }, created)
}

/// `0x0067a730`: verify a virtual setter by reading the value back.
///
/// The retail body writes `wanted` through vtable slot `+8`, reads it back
/// through slot `+4`, and returns `0x14` when the read-back differs from
/// `wanted`, `0` when it matches.
pub fn w14a06_verify_set_value(readback: u32, wanted: u32) -> u32 {
    if readback == wanted { 0 } else { 0x14 }
}

// ---------------------------------------------------------------------------
// WolfeLimiter
// ---------------------------------------------------------------------------

/// Section and key names read by `FUN_0067ad50`.
pub const W14A06_LIMITER_SECTION: &str = "WolfeLimiter";
/// Release-time key (`paramReleaseMs`, length `0x0e`).
pub const W14A06_LIMITER_RELEASE_KEY: &str = "paramReleaseMs";
/// ARC-enable key (`paramUseARC`, length `0x0b`).
pub const W14A06_LIMITER_ARC_KEY: &str = "paramUseARC";

/// `0x0067ad50`: reload the two limiter settings exposed in the config file.
///
/// The retail body reads `paramReleaseMs` (default: the current `+0x64`
/// value) and `paramUseARC` (default: the current `+0x68` byte) from the
/// `WolfeLimiter` section, in that order.  The two lookups are abstracted as
/// caller-supplied closures.
pub fn w14a06_limiter_read_config<F, G>(
    release_ms: i32,
    use_arc: bool,
    read_int: F,
    read_bool: G,
) -> (i32, bool)
where
    F: FnOnce(&str, &str, i32) -> i32,
    G: FnOnce(&str, &str, bool) -> bool,
{
    let release = read_int(
        W14A06_LIMITER_SECTION,
        W14A06_LIMITER_RELEASE_KEY,
        release_ms,
    );
    let arc = read_bool(W14A06_LIMITER_SECTION, W14A06_LIMITER_ARC_KEY, use_arc);
    (release, arc)
}

/// The "WolfeLimiter" object.
///
/// The running envelope (`+0x04..=0x40`) is driven by `FUN_00679a40`, which
/// lies before this window; the fields below record everything the setup and
/// block-scaling rows read or write.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct W14a06Limiter {
    /// `+0x00`: word before the envelope state (not touched here).
    pub reserved_00: u32,
    /// `+0x04`: envelope state base (not touched here).
    pub reserved_04: u32,
    /// `+0x08`: release one-pole coefficient.
    pub release_coefficient: f32,
    /// `+0x0c`: cached `release_ms`.
    pub release_ms_cache: i32,
    /// `+0x10`: cached sample rate.
    pub sample_rate_cache: i32,
    /// `+0x14`: current envelope gain (driven outside this window).
    pub envelope_state: f32,
    /// `+0x18`: second one-pole coefficient.
    pub second_coefficient: f32,
    /// `+0x1c`: cached `second_ms`.
    pub second_ms_cache: i32,
    /// `+0x20`: cached sample rate for the second coefficient.
    pub second_rate_cache: i32,
    /// `+0x24`: third one-pole coefficient.
    pub third_coefficient: f32,
    /// `+0x28`: cached `third_ms`.
    pub third_ms_cache: i32,
    /// `+0x2c`: copy of the `paramUseARC` byte.
    pub use_arc_copy: u8,
    /// `+0x2d..+0x2f`: padding.
    pub reserved_2d: [u8; 3],
    /// `+0x30`: envelope limit (driven outside this window).
    pub reserved_30: f32,
    /// `+0x34`: `10^(threshold_db/20)`.
    pub threshold_gain: f32,
    /// `+0x38`: `param_80` as `f32`.
    pub param_80_f32: f32,
    /// `+0x3c`: `param_84` as `f32`.
    pub param_84_f32: f32,
    /// `+0x40`: `(param_84 - param_80) / (rate * ramp_ms * 0.001)`.
    pub ramp_per_sample: f32,
    /// `+0x44`: `10^((threshold_db - offset_db)/20)`, lower knee.
    pub lower_knee_gain: f32,
    /// `+0x48`: `10^(threshold_db/20)`, upper knee.
    pub upper_knee_gain: f32,
    /// `+0x4c`: `threshold_db - offset_db`.
    pub threshold_minus_offset: f32,
    /// `+0x50`: copy of `threshold_db`.
    pub threshold_db_copy: f32,
    /// `+0x54`: `0 < offset_db`.
    pub enabled: bool,
    /// `+0x55..+0x57`: padding.
    pub reserved_55: [u8; 3],
    /// `+0x58`: `10^(makeup_db/20)`.
    pub makeup_gain: f32,
    /// `+0x5c`: `makeup_db != 0`.
    pub makeup_present: bool,
    /// `+0x5d..+0x5f`: padding.
    pub reserved_5d: [u8; 3],
    /// `+0x60`: another `10^(threshold_db/20)` copy.
    pub threshold_gain_copy: f32,
    /// `+0x64`: `paramReleaseMs`.
    pub release_ms: i32,
    /// `+0x68`: `paramUseARC`.
    pub use_arc: bool,
    /// `+0x69..+0x6b`: padding.
    pub reserved_69: [u8; 3],
    /// `+0x6c`: threshold in decibels.
    pub threshold_db: f32,
    /// `+0x70`: makeup gain in decibels.
    pub makeup_db: f32,
    /// `+0x74`: knee offset in decibels.
    pub offset_db: f32,
    /// `+0x78`: third time constant in milliseconds.
    pub third_ms: i32,
    /// `+0x7c`: second time constant in milliseconds.
    pub second_ms: i32,
    /// `+0x80`: ramp start value.
    pub param_80: i32,
    /// `+0x84`: ramp end value.
    pub param_84: i32,
    /// `+0x88`: ramp duration in milliseconds.
    pub ramp_ms: i32,
}

/// One-pole coefficient `exp(-2 / (ms * rate * 0.001))`, or `0` when the
/// time constant is not positive.
fn w14a06_decay_coefficient(milliseconds: i32, sample_rate: i32) -> f32 {
    if milliseconds < 1 {
        0.0
    } else {
        (-2.0f32 / (milliseconds as f32 * sample_rate as f32 * 0.001)).exp()
    }
}

/// `10^(db/20)`.
fn w14a06_db_to_gain(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

/// `0x0067bd80`: recompute the limiter's derived coefficients and gains.
///
/// The two cached time constants (`+0x0c`/`+0x10` and `+0x1c`/`+0x20`) are
/// only recomputed when their inputs change; the third coefficient is
/// recomputed whenever `third_ms` changes.  The decibel gains are always
/// refreshed.  `sample_rate` is the `FUN_0067bd80` argument.
pub fn w14a06_limiter_update(limiter: &mut W14a06Limiter, sample_rate: i32) {
    if limiter.release_ms_cache != limiter.release_ms || limiter.sample_rate_cache != sample_rate {
        limiter.release_ms_cache = limiter.release_ms;
        limiter.sample_rate_cache = sample_rate;
        limiter.release_coefficient = w14a06_decay_coefficient(limiter.release_ms, sample_rate);
    }
    limiter.use_arc_copy = limiter.use_arc as u8;
    limiter.threshold_gain = w14a06_db_to_gain(limiter.threshold_db);
    limiter.param_80_f32 = limiter.param_80 as f32;
    limiter.param_84_f32 = limiter.param_84 as f32;
    limiter.ramp_per_sample = (limiter.param_84 - limiter.param_80) as f32
        / (limiter.ramp_ms as f32 * sample_rate as f32 * 0.001);
    if limiter.second_ms_cache != limiter.second_ms || limiter.second_rate_cache != sample_rate {
        limiter.second_ms_cache = limiter.second_ms;
        limiter.second_rate_cache = sample_rate;
        limiter.second_coefficient = w14a06_decay_coefficient(limiter.second_ms, sample_rate);
    }
    if limiter.third_ms_cache != limiter.third_ms {
        limiter.third_ms_cache = limiter.third_ms;
        limiter.third_coefficient = w14a06_decay_coefficient(limiter.third_ms, sample_rate);
    }
    limiter.threshold_db_copy = limiter.threshold_db;
    limiter.enabled = 0.0 < limiter.offset_db;
    limiter.threshold_minus_offset = limiter.threshold_db - limiter.offset_db;
    limiter.upper_knee_gain = w14a06_db_to_gain(limiter.threshold_db);
    limiter.lower_knee_gain = w14a06_db_to_gain(limiter.threshold_minus_offset);
    limiter.threshold_gain_copy = limiter.upper_knee_gain;
    limiter.makeup_present = limiter.makeup_db != 0.0;
    limiter.makeup_gain = w14a06_db_to_gain(limiter.makeup_db);
}

/// `0x0067b3c0`: choose the gain for one mixed block and whether the source
/// must be copied first.
///
/// Three arms, in retail order:
///
/// * ARC enabled (`enabled`) and `gain` strictly between the two knees:
///   `10^(((20*log10(g) - (threshold - offset)) / offset) *
///   (threshold - 20*log10(g)) / 20) * makeup`.
/// * `gain` above the threshold knee: `10^((threshold - 20*log10(g))/20) *
///   makeup`.
/// * otherwise the makeup gain alone, and the source is copied when
///   `makeup_db == 0` (the retail `+0x5c` byte is clear).
pub fn w14a06_limiter_block_factor(limiter: &W14a06Limiter, gain: f32) -> (f32, bool) {
    if limiter.enabled && gain > limiter.lower_knee_gain && gain < limiter.upper_knee_gain {
        let decibels = 20.0 * gain.log10();
        let exponent = ((decibels - limiter.threshold_minus_offset) / limiter.offset_db)
            * (limiter.threshold_db - decibels)
            / 20.0;
        (10.0f32.powf(exponent) * limiter.makeup_gain, false)
    } else if gain > limiter.threshold_gain_copy {
        let exponent = (limiter.threshold_db - 20.0 * gain.log10()) / 20.0;
        (10.0f32.powf(exponent) * limiter.makeup_gain, false)
    } else {
        (limiter.makeup_gain, !limiter.makeup_present)
    }
}

/// `0x0067b3c0`: scale `blocks` sample blocks through the limiter.
///
/// `gains` supplies the value `FUN_00679a40` would return for each block
/// (that routine owns the envelope and lies outside the window).  `count` is
/// the number of `f32` samples per block; the retail body requires the two
/// caller count arguments to be equal, which this single parameter encodes.
/// Returns the number of blocks processed, or `None` when the row's null /
/// zero-count guards reject the call or the slices are too short.
pub fn w14a06_limiter_process(
    limiter: &W14a06Limiter,
    gains: &[f32],
    src: &[f32],
    dst: &mut [f32],
    count: usize,
    blocks: usize,
) -> Option<usize> {
    if blocks == 0 || gains.len() < blocks {
        return None;
    }
    let total = count.checked_mul(blocks)?;
    if src.len() < total || dst.len() < total {
        return None;
    }
    for (block, &gain) in gains.iter().take(blocks).enumerate() {
        let (factor, copy_first) = w14a06_limiter_block_factor(limiter, gain);
        let start = block * count;
        let end = start + count;
        if copy_first {
            dst[start..end].copy_from_slice(&src[start..end]);
        }
        for sample in &mut dst[start..end] {
            *sample *= factor;
        }
    }
    Some(blocks)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave14_agent06 {
    use super::*;

    fn approx(left: f32, right: f32) -> bool {
        (left - right).abs() <= 1e-5 * right.abs().max(1.0)
    }

    fn table() -> Vec<W14a06Channel> {
        vec![W14a06Channel::cleared(); W14A06_MAX_CHANNELS]
    }

    fn channel_at(generation: u16, buffer: u32) -> W14a06Channel {
        W14a06Channel {
            buffer,
            generation,
            ..W14a06Channel::cleared()
        }
    }

    #[test]
    fn channel_constructor_layout_is_the_retail_initialiser() {
        let channel = W14a06Channel::cleared();
        assert_eq!(channel.buffer, 0);
        assert_eq!(channel.generation, 0);
        assert_eq!(channel.flags_06, 0);
        assert!(!channel.valid);
        assert_eq!(channel.data, 0);
        assert_eq!(channel.pending, W14A06_INVALID_HANDLE);
        assert_eq!(channel.volume, 0.0);
        assert_eq!(channel.pan, 0.0);
        assert_eq!(W14A06_CHANNEL_STRIDE, 0x18);
        assert_eq!(W14A06_CHANNEL_TABLE_OFFSET, 0x10);
    }

    #[test]
    fn release_channel_advances_generation_and_clears_pending() {
        let mut channel = W14a06Channel {
            buffer: 0x1234,
            generation: 7,
            flags_06: 3,
            valid: true,
            data: 0x5678,
            pending: 0x0007_0002,
            volume: 0.25,
            pan: -0.5,
        };
        assert_eq!(w14a06_release_channel(&mut channel), 0x1234);
        assert_eq!(channel.buffer, 0);
        assert_eq!(channel.generation, 8);
        assert_eq!(channel.flags_06, 0);
        assert!(!channel.valid);
        assert_eq!(channel.data, 0);
        assert_eq!(channel.pending, W14A06_INVALID_HANDLE);
        assert_eq!(channel.volume, 0.25);
        assert_eq!(channel.pan, -0.5);

        assert_eq!(w14a06_next_generation(0), 1);
        assert_eq!(w14a06_next_generation(1), 2);
        assert_eq!(w14a06_next_generation(0xffff), 0);
        assert_eq!(w14a06_next_generation(0xfffe), 0xffff);
    }

    #[test]
    fn release_handle_checks_index_generation_and_zero_generation() {
        let mut channels = table();
        channels[3] = channel_at(7, 0xabc);
        let handle = (7u32 << 16) | 3;
        let released = w14a06_release_channel_handle(&mut channels, handle).unwrap();
        assert_eq!(released.index, 3);
        assert_eq!(released.buffer, 0xabc);
        assert_eq!(channels[3].generation, 8);
        assert_eq!(channels[3].buffer, 0);

        assert!(w14a06_release_channel_handle(&mut channels, handle).is_none());
        assert!(w14a06_release_channel_handle(&mut channels, 0x0007_0080).is_none());
        assert!(w14a06_release_channel_handle(&mut channels, 0x0000_0003).is_none());
        assert!(w14a06_release_channel_handle(&mut channels, 0x0009_0003).is_none());
    }

    #[test]
    fn find_channel_ignores_the_in_use_marker() {
        let mut channels = table();
        channels[5] = channel_at(0x20, 0);
        assert!(!channels[5].valid);
        assert_eq!(w14a06_find_channel(&channels, 0x0020_0005), Some(5));
        assert_eq!(w14a06_find_channel(&channels, 0x0021_0005), None);
        assert_eq!(w14a06_find_channel(&channels, 0x0020_0006), None);
        assert_eq!(w14a06_find_channel(&channels, 0x0000_0005), None);
        assert_eq!(w14a06_find_channel(&channels, 0x0020_0080), None);
        assert!(w14a06_channel_has_data(&W14a06Channel {
            data: 1,
            ..W14a06Channel::cleared()
        }));
    }

    #[test]
    fn finished_query_covers_flag_buffer_and_backend_arms() {
        let mut channels = table();
        let handle = (4u32 << 16) | 1;
        channels[1] = channel_at(4, 0);
        // Live handle, no buffer, no pending data: finished.
        assert!(w14a06_channel_is_finished(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(false)
        ));
        channels[1].data = 9;
        assert!(!w14a06_channel_is_finished(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(false)
        ));
        // The +0x06 byte short-circuits every other arm.
        channels[1].flags_06 = 1;
        assert!(!w14a06_channel_is_finished(
            &mut channels,
            handle,
            W14a06BackendQuery::Dead
        ));
        channels[1].flags_06 = 0;
        channels[1].buffer = 0x55;
        assert!(w14a06_channel_is_finished(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(false)
        ));
        assert!(!w14a06_channel_is_finished(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(true)
        ));
        assert!(w14a06_channel_is_finished(
            &mut channels,
            handle,
            W14a06BackendQuery::Dead
        ));
        assert_eq!(channels[1].buffer, 0);
        assert_eq!(channels[1].generation, 5);
        // Bad handle answers finished.
        assert!(w14a06_channel_is_finished(
            &mut channels,
            0xffff_ffff,
            W14a06BackendQuery::Value(true)
        ));
    }

    #[test]
    fn playing_query_returns_status_and_recycles_dead_buffers() {
        let mut channels = table();
        let handle = (9u32 << 16) | 2;
        channels[2] = channel_at(9, 0);
        channels[2].data = 0x77;
        assert!(w14a06_channel_is_playing(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(false)
        ));
        channels[2].data = 0;
        channels[2].buffer = 0x99;
        assert!(w14a06_channel_is_playing(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(true)
        ));
        assert!(!w14a06_channel_is_playing(
            &mut channels,
            handle,
            W14a06BackendQuery::Value(false)
        ));
        assert!(!w14a06_channel_is_playing(
            &mut channels,
            handle,
            W14a06BackendQuery::Dead
        ));
        assert_eq!(channels[2].buffer, 0);
        assert_eq!(channels[2].generation, 10);
        assert!(!w14a06_channel_is_playing(
            &mut channels,
            0x0009_007f + 1,
            W14a06BackendQuery::Value(true)
        ));
    }

    #[test]
    fn dword_query_returns_value_or_recycles() {
        let mut channel = channel_at(3, 0x44);
        assert_eq!(
            w14a06_channel_query(&mut channel, W14a06BackendQuery::Value(0x1234)),
            0x1234
        );
        channel.buffer = 0;
        assert_eq!(
            w14a06_channel_query(&mut channel, W14a06BackendQuery::Value(0x1234)),
            0
        );
        channel.buffer = 0x44;
        assert_eq!(
            w14a06_channel_query(&mut channel, W14a06BackendQuery::Dead),
            0
        );
        assert_eq!(channel.buffer, 0);
        assert_eq!(channel.generation, 4);
        assert!(matches!(
            W14a06BackendQuery::<u8>::from_status(0x24, 1),
            W14a06BackendQuery::Dead
        ));
        assert!(matches!(
            W14a06BackendQuery::<u8>::from_status(0x0b, 1),
            W14a06BackendQuery::Dead
        ));
        assert!(matches!(
            W14a06BackendQuery::<u8>::from_status(0, 1),
            W14a06BackendQuery::Value(1)
        ));
    }

    #[test]
    fn pan_and_volume_setters_choose_deferred_or_live_path() {
        let mut channels = table();
        let handle = (6u32 << 16) | 4;
        channels[4] = channel_at(6, 0);
        assert!(!w14a06_set_channel_pan(&mut channels, handle, 0.5, true));
        assert!(!w14a06_set_channel_volume(&mut channels, handle, 0.5, true));
        channels[4].buffer = 0x12;
        assert!(w14a06_set_channel_pan(&mut channels, handle, 0.5, true));
        assert!(!w14a06_set_channel_pan(&mut channels, handle, 0.5, false));
        assert!(w14a06_set_channel_volume(&mut channels, handle, 0.25, true));
        assert!(!w14a06_set_channel_volume(
            &mut channels,
            handle,
            0.25,
            false
        ));
        channels[4].data = 0x88;
        assert!(w14a06_set_channel_pan(&mut channels, handle, -0.75, false));
        assert_eq!(channels[4].pan, -0.75);
        assert!(w14a06_set_channel_volume(
            &mut channels,
            handle,
            0.625,
            false
        ));
        assert_eq!(channels[4].volume, 0.625);
        assert!(!w14a06_set_channel_pan(
            &mut channels,
            0x0006_0005,
            1.0,
            true
        ));
    }

    #[test]
    fn group_volume_targets_the_recovered_offsets() {
        let groups = [0x11u32, 0x22, 0x33];
        assert_eq!(
            w14a06_set_group_volume(0x99, &groups, 0, 0.5),
            Some((0x11, 0.5))
        );
        assert_eq!(
            w14a06_set_group_volume(0x99, &groups, 2, 1.0),
            Some((0x33, 1.0))
        );
        assert_eq!(w14a06_set_group_volume(0, &groups, 1, 1.0), None);
        assert_eq!(w14a06_set_group_volume(0x99, &groups, 3, 1.0), None);
    }

    #[test]
    fn voice_and_stream_release_report_handles_in_order() {
        let mut channels = table();
        channels[7] = channel_at(2, 0x2000);
        let mut voice = W14a06Voice {
            buffer: 0x3000,
            reserved: 0,
            handle: (2u32 << 16) | 7,
        };
        let released = w14a06_release_voice(&mut voice, &mut channels);
        assert_eq!(released.buffer, 0x3000);
        assert_eq!(released.handle.unwrap().index, 7);
        assert_eq!(released.handle.unwrap().buffer, 0x2000);
        assert_eq!(voice.buffer, 0);
        assert_eq!(channels[7].generation, 3);

        channels[8] = channel_at(5, 0x4000);
        let mut stream = W14a06Stream {
            buffer: 0x5000,
            memory: 0x6000,
            handle: (5u32 << 16) | 8,
            text: 0x7000,
            text_len: 0x40,
            ..W14a06Stream::default()
        };
        let released = w14a06_release_stream(&mut stream, &mut channels);
        assert_eq!(released.buffer, 0x6000);
        assert_eq!(released.text, Some((0x7000, 0x40)));
        assert_eq!(released.handle.unwrap().index, 8);
        assert_eq!(stream.memory, 0);
        assert_eq!(stream.text_len, 0);
        assert_eq!(stream.text, 0x7000);
    }

    #[test]
    fn close_sound_releases_groups_limiter_then_device() {
        let mut handles = [0x10u32, 0x20, 0x30, 0x40];
        let mut limiter = 0x50u32;
        let actions = w14a06_close_sound(&mut handles, &mut limiter).unwrap();
        assert_eq!(actions.groups, [0x20, 0x30, 0x40]);
        assert_eq!(actions.limiter, 0x50);
        assert_eq!(actions.device, 0x10);
        assert_eq!(handles, [0, 0, 0, 0]);
        assert_eq!(limiter, 0);
        assert!(w14a06_close_sound(&mut handles, &mut limiter).is_none());
    }

    #[test]
    fn channel_play_maps_every_query_arm() {
        let mut channel = channel_at(1, 0);
        assert!(!w14a06_channel_play(
            &mut channel,
            0,
            W14a06BackendQuery::Value(true),
            0
        ));
        channel.buffer = 0x10;
        assert!(w14a06_channel_play(
            &mut channel,
            0,
            W14a06BackendQuery::Value(true),
            0
        ));
        assert!(!w14a06_channel_play(
            &mut channel,
            7,
            W14a06BackendQuery::Value(true),
            1
        ));
        channel.flags_06 = 1;
        assert!(w14a06_channel_play(
            &mut channel,
            0,
            W14a06BackendQuery::Value(false),
            0
        ));
        assert_eq!(channel.flags_06, 0);
        channel.buffer = 0x20;
        channel.flags_06 = 1;
        assert!(w14a06_channel_play(
            &mut channel,
            0,
            W14a06BackendQuery::Dead,
            0
        ));
        assert_eq!(channel.buffer, 0);
        assert_eq!(channel.generation, 2);
    }

    #[test]
    fn stream_prepare_parses_once_and_keeps_failed_allocation_zero() {
        let mut data = W14a06SoundData {
            source: 0x10,
            ..W14a06SoundData::default()
        };
        let probe = W14a06PrepareProbe {
            parse_status: 1,
            parsed: 0x55,
            alloc_status: 0,
            allocated: 0x66,
        };
        assert!(!w14a06_stream_prepare(&mut data, probe));
        assert_eq!(data.prepared, 0);
        let probe = W14a06PrepareProbe {
            parse_status: 0,
            parsed: 0,
            alloc_status: 1,
            allocated: 0x66,
        };
        assert!(w14a06_stream_prepare(&mut data, probe));
        assert_eq!(data.prepared, 0);
        assert_eq!(data.group, 2);
        assert_eq!(data.samples, 0);
        data.prepared = 0x77;
        let probe = W14a06PrepareProbe {
            parse_status: 9,
            parsed: 9,
            alloc_status: 9,
            allocated: 9,
        };
        assert!(w14a06_stream_prepare(&mut data, probe));
        assert_eq!(data.prepared, 0x77);
    }

    #[test]
    fn play_on_channel_starts_or_defers_a_voice() {
        let sound = W14a06Sound {
            device: 0x99,
            groups: [0x11, 0x22, 0x33],
        };
        let mut channels = table();
        channels[3] = channel_at(6, 0);
        let mut data = W14a06SoundData {
            source: 0x10,
            prepared: 1,
            samples: 0x100,
            group: 1,
        };
        let probe = W14a06PlayProbe {
            prepare: W14a06PrepareProbe::default(),
            create_status: 0,
            created_buffer: 0xabc,
        };
        let result = w14a06_play_on_channel(
            &sound,
            &mut channels,
            &mut data,
            0xdead,
            (6u32 << 16) | 3,
            0.5,
            -0.25,
            probe,
        );
        assert_eq!(
            result,
            W14a06PlayResult::Started {
                index: 3,
                group: 0x22
            }
        );
        // The immediate path applies volume and pan to the buffer only; the
        // record fields stay at the zeroes the row wrote before the create.
        assert_eq!(channels[3].buffer, 0xabc);
        assert_eq!(channels[3].volume, 0.0);
        assert_eq!(channels[3].pan, 0.0);
        assert!(channels[3].valid);
        assert_eq!(channels[3].data, 0);

        let deferred = W14a06PlayProbe {
            create_status: W14A06_DEFERRED_CREATE_STATUS,
            created_buffer: 0,
            ..W14a06PlayProbe::default()
        };
        let result = w14a06_play_on_channel(
            &sound,
            &mut channels,
            &mut data,
            0xbeef,
            (6u32 << 16) | 3,
            0.25,
            0.75,
            deferred,
        );
        assert_eq!(result, W14a06PlayResult::Deferred { index: 3 });
        assert_eq!(channels[3].data, 0xbeef);
        assert_eq!(channels[3].pending, (6u32 << 16) | 3);
        assert_eq!(channels[3].volume, 0.25);
        assert_eq!(channels[3].pan, 0.75);

        let failed = W14a06PlayProbe {
            create_status: 5,
            created_buffer: 0x55,
            ..W14a06PlayProbe::default()
        };
        assert_eq!(
            w14a06_play_on_channel(
                &sound,
                &mut channels,
                &mut data,
                0,
                (6u32 << 16) | 3,
                1.0,
                0.0,
                failed,
            ),
            W14a06PlayResult::Failed
        );
        assert_eq!(
            w14a06_play_on_channel(
                &sound,
                &mut channels,
                &mut data,
                0,
                0x0005_0003,
                1.0,
                0.0,
                failed,
            ),
            W14a06PlayResult::InvalidHandle
        );
    }

    #[test]
    fn play_sound_requires_a_device_then_forwards() {
        let mut channels = table();
        channels[1] = channel_at(2, 0);
        let mut data = W14a06SoundData {
            prepared: 1,
            group: 2,
            ..W14a06SoundData::default()
        };
        let no_device = W14a06Sound::default();
        assert_eq!(
            w14a06_play_sound(
                &no_device,
                &mut channels,
                &mut data,
                1,
                0.0,
                0.0,
                0x0002_0001,
                W14a06PlayProbe::default()
            ),
            W14a06PlayResult::NoDevice
        );
        let sound = W14a06Sound {
            device: 5,
            groups: [1, 2, 3],
        };
        assert_eq!(
            w14a06_play_sound(
                &sound,
                &mut channels,
                &mut data,
                1,
                0.5,
                0.0,
                0x0002_0001,
                W14a06PlayProbe {
                    create_status: 0,
                    created_buffer: 0x80,
                    ..W14a06PlayProbe::default()
                }
            ),
            W14a06PlayResult::Started { index: 1, group: 3 }
        );
        assert_eq!(channels[1].buffer, 0x80);
    }

    #[test]
    fn sample_and_stream_play_validate_handle_and_expose_records() {
        let sound = W14a06Sound {
            device: 1,
            groups: [0, 0, 0],
        };
        let mut channels = table();
        channels[2] = channel_at(3, 0);
        let mut sample = W14a06Sample {
            samples: 0x4000,
            group: 1,
            handle: 0,
        };
        assert!(w14a06_play_sample(
            &sound,
            &mut channels,
            &mut sample,
            0.5,
            -0.5,
            (3u32 << 16) | 2,
            0,
            0x9000
        ));
        assert_eq!(sample.handle, (3u32 << 16) | 2);
        assert_eq!(channels[2].buffer, 0x9000);
        assert_eq!(channels[2].volume, 0.5);
        assert_eq!(channels[2].pan, -0.5);
        assert!(channels[2].valid);
        let mut sample2 = W14a06Sample {
            samples: 1,
            group: 0,
            handle: 0,
        };
        assert!(!w14a06_play_sample(
            &sound,
            &mut channels,
            &mut sample2,
            1.0,
            0.0,
            (4u32 << 16) | 2,
            1,
            0x9000
        ));
        assert!(!w14a06_play_sample(
            &W14a06Sound::default(),
            &mut channels,
            &mut sample2,
            1.0,
            0.0,
            (3u32 << 16) | 2,
            0,
            0x9000
        ));

        let mut stream = W14a06StreamData {
            samples: 0x5000,
            group: 2,
            ..W14a06StreamData::default()
        };
        assert!(w14a06_play_stream(
            &sound,
            &mut channels,
            &mut stream,
            0.75,
            0.25,
            (3u32 << 16) | 2,
            0,
            0xa000
        ));
        assert_eq!(stream.handle, (3u32 << 16) | 2);
        assert_eq!(channels[2].buffer, 0xa000);
        assert_eq!(channels[2].volume, 0.75);
        assert_eq!(channels[2].pan, 0.25);
    }

    #[test]
    fn interface_result_mappings_use_the_recovered_codes() {
        assert_eq!(w14a06_map_create_result(0), (0, 0));
        assert_eq!(w14a06_map_create_result(7), (0x16, 7));
        assert_eq!(w14a06_verify_set_value(5, 5), 0);
        assert_eq!(w14a06_verify_set_value(5, 6), 0x14);
        assert_eq!(w14a06_verify_set_value(0, 0), 0);
    }

    #[test]
    fn limiter_config_reads_release_then_arc_with_defaults() {
        let mut calls: Vec<(String, String, i32)> = Vec::new();
        let (release, arc) = w14a06_limiter_read_config(
            40,
            true,
            |section, key, default| {
                calls.push((section.to_string(), key.to_string(), default));
                55
            },
            |section, key, default| {
                assert_eq!(section, "WolfeLimiter");
                assert_eq!(key, "paramUseARC");
                assert!(default);
                false
            },
        );
        assert_eq!(release, 55);
        assert!(!arc);
        assert_eq!(
            calls,
            vec![("WolfeLimiter".to_string(), "paramReleaseMs".to_string(), 40)]
        );
        let (release, arc) =
            w14a06_limiter_read_config(12, false, |_, _, default| default, |_, _, default| default);
        assert_eq!((release, arc), (12, false));
        assert_eq!(W14A06_LIMITER_RELEASE_KEY.len(), 0x0e);
        assert_eq!(W14A06_LIMITER_ARC_KEY.len(), 0x0b);
        assert_eq!(W14A06_LIMITER_SECTION.len(), 0x0c);
    }

    #[test]
    fn limiter_update_recomputes_coefficients_and_gains() {
        let mut limiter = W14a06Limiter {
            release_ms: 100,
            second_ms: 50,
            third_ms: 200,
            threshold_db: 0.0,
            offset_db: 6.0,
            makeup_db: 10.0,
            param_80: 0,
            param_84: 100,
            ramp_ms: 10,
            ..W14a06Limiter::default()
        };
        w14a06_limiter_update(&mut limiter, 1000);
        assert!(approx(limiter.release_coefficient, (-0.02f32).exp()));
        assert!(approx(limiter.second_coefficient, (-0.04f32).exp()));
        assert!(approx(limiter.third_coefficient, (-0.01f32).exp()));
        assert_eq!(limiter.release_ms_cache, 100);
        assert_eq!(limiter.sample_rate_cache, 1000);
        assert_eq!(limiter.use_arc_copy, 0);
        assert!(approx(limiter.threshold_gain, 1.0));
        assert_eq!(limiter.param_80_f32, 0.0);
        assert_eq!(limiter.param_84_f32, 100.0);
        assert!(approx(limiter.ramp_per_sample, 100.0 / 10.0));
        assert!(limiter.enabled);
        assert!(approx(limiter.threshold_minus_offset, -6.0));
        assert!(approx(limiter.upper_knee_gain, 1.0));
        assert!(approx(limiter.lower_knee_gain, 10f32.powf(-0.3)));
        assert!(approx(limiter.threshold_gain_copy, 1.0));
        assert!(limiter.makeup_present);
        assert!(approx(limiter.makeup_gain, 10f32.powf(0.5)));
        assert!(approx(limiter.threshold_db_copy, 0.0));

        // Unchanged caches keep the coefficients identical.
        let release_before = limiter.release_coefficient;
        limiter.release_ms = 100;
        w14a06_limiter_update(&mut limiter, 1000);
        assert_eq!(limiter.release_coefficient, release_before);

        // A negative time constant yields zero.
        limiter.release_ms = 0;
        w14a06_limiter_update(&mut limiter, 1000);
        assert_eq!(limiter.release_coefficient, 0.0);

        // Disabled ARC when the offset is not positive.
        limiter.offset_db = 0.0;
        limiter.makeup_db = 0.0;
        w14a06_limiter_update(&mut limiter, 22050);
        assert!(!limiter.enabled);
        assert!(!limiter.makeup_present);
        assert!(approx(limiter.makeup_gain, 1.0));
        assert_eq!(limiter.use_arc_copy, 0);
    }

    #[test]
    fn limiter_block_factor_uses_arc_knee_and_makeup_arms() {
        let mut limiter = W14a06Limiter {
            threshold_db: 0.0,
            offset_db: 6.0,
            makeup_db: 10.0,
            ..W14a06Limiter::default()
        };
        w14a06_limiter_update(&mut limiter, 1000);
        // Below the lower knee: makeup only and no copy (makeup present).
        let (factor, copy) = w14a06_limiter_block_factor(&limiter, 0.25);
        assert!(approx(factor, limiter.makeup_gain));
        assert!(!copy);
        // Above the upper knee: hard threshold gain.
        let (factor, copy) = w14a06_limiter_block_factor(&limiter, 4.0);
        let expected = 10f32.powf((0.0 - 20.0 * 4.0f32.log10()) / 20.0) * limiter.makeup_gain;
        assert!(approx(factor, expected));
        assert!(!copy);
        // Between the knees: ARC formula.
        let gain = 0.75;
        let (factor, copy) = w14a06_limiter_block_factor(&limiter, gain);
        let db = 20.0 * gain.log10();
        let exponent = ((db - limiter.threshold_minus_offset) / limiter.offset_db)
            * (limiter.threshold_db - db)
            / 20.0;
        assert!(approx(factor, 10f32.powf(exponent) * limiter.makeup_gain));
        assert!(!copy);

        // With no makeup gain the below-knee path requests a copy.
        limiter.makeup_db = 0.0;
        w14a06_limiter_update(&mut limiter, 1000);
        let (factor, copy) = w14a06_limiter_block_factor(&limiter, 0.25);
        assert!(approx(factor, 1.0));
        assert!(copy);
    }

    #[test]
    fn limiter_process_scales_each_block_with_its_gain() {
        let mut limiter = W14a06Limiter {
            threshold_db: 0.0,
            offset_db: 6.0,
            makeup_db: 0.0,
            ..W14a06Limiter::default()
        };
        w14a06_limiter_update(&mut limiter, 1000);
        let src = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        // The hard-threshold arm scales the destination in place (the retail
        // caller feeds an already mixed block); prefill it with the source.
        let mut dst = src;
        // First block: gain 0.25 -> copy + unity; second: gain 4.0 -> hard knee.
        let processed =
            w14a06_limiter_process(&limiter, &[0.25, 4.0], &src, &mut dst, 3, 2).unwrap();
        assert_eq!(processed, 2);
        assert_eq!(&dst[0..3], &[1.0, 2.0, 3.0]);
        let hard = 10f32.powf((0.0 - 20.0 * 4.0f32.log10()) / 20.0);
        assert!(approx(dst[3], 4.0 * hard));
        assert!(approx(dst[4], 5.0 * hard));
        assert!(approx(dst[5], 6.0 * hard));

        // Guard: zero blocks, too few gains, short slices.
        assert_eq!(
            w14a06_limiter_process(&limiter, &[], &src, &mut dst, 3, 0),
            None
        );
        assert_eq!(
            w14a06_limiter_process(&limiter, &[1.0], &src, &mut dst, 3, 2),
            None
        );
        assert_eq!(
            w14a06_limiter_process(&limiter, &[1.0, 1.0], &src[..5], &mut dst, 3, 2),
            None
        );
    }
}

// ===========================================================================
// Merged from src/wave31/agent01.rs
// ===========================================================================
// Wave 31 agent 01: command packet builders and a table lookup.

/// Builds a 2-byte command packet `[opcode, arg]`.
fn w31a01_pkt2(opcode: u8, arg: u8) -> [u8; 2] {
    [opcode, arg]
}

/// Builds a 3-byte command packet `[opcode, arg_lo, arg_hi]`.
fn w31a01_pkt3(opcode: u8, arg: u16) -> [u8; 3] {
    let b = arg.to_le_bytes();
    [opcode, b[0], b[1]]
}

/// `0x00572120`: 1-based index of `wanted` in the table, scanning while the
/// index is below `0x2f3` (or `0x3e7` when `extended`); 0 when absent or 0.
pub fn w31a01_find_index(table: &[u32], wanted: u32, extended: bool) -> u32 {
    if wanted == 0 {
        return 0;
    }
    let limit: u32 = if extended { 0xf4 } else { 0 } + 0x2f3;
    let mut i: u32 = 0;
    loop {
        let next = i + 1;
        if table.get(i as usize).copied() == Some(wanted) {
            return next;
        }
        i = next;
        if next >= limit {
            return 0;
        }
    }
}

/// `0x0072baf0`: packet opcode 0x1f with a 16-bit argument (3 bytes).
pub fn w31a01_pkt_1f(arg: u16) -> [u8; 3] {
    w31a01_pkt3(0x1f, arg)
}
/// `0x0072c520`: packet opcode 0x2d with a byte argument (2 bytes).
pub fn w31a01_pkt_2d(arg: u8) -> [u8; 2] {
    w31a01_pkt2(0x2d, arg)
}
/// `0x0072c540`: packet opcode 0x22 with a byte argument (2 bytes).
pub fn w31a01_pkt_22(arg: u8) -> [u8; 2] {
    w31a01_pkt2(0x22, arg)
}
/// `0x0072c920`: packet opcode 0x1e with a byte argument (2 bytes).
pub fn w31a01_pkt_1e(arg: u8) -> [u8; 2] {
    w31a01_pkt2(0x1e, arg)
}
/// `0x0072e6f0`: packet opcode 0x25 with a byte argument (2 bytes).
pub fn w31a01_pkt_25(arg: u8) -> [u8; 2] {
    w31a01_pkt2(0x25, arg)
}
/// `0x0072e760`: packet opcode 0x28 with a byte argument (2 bytes).
pub fn w31a01_pkt_28(arg: u8) -> [u8; 2] {
    w31a01_pkt2(0x28, arg)
}
/// `0x0072e780`: packet opcode 0x32 with a byte argument (2 bytes).
pub fn w31a01_pkt_32(arg: u8) -> [u8; 2] {
    w31a01_pkt2(0x32, arg)
}
/// `0x0072e7a0`: packet opcode 0x35 with a 16-bit argument (3 bytes).
pub fn w31a01_pkt_35(arg: u16) -> [u8; 3] {
    w31a01_pkt3(0x35, arg)
}

#[cfg(test)]
mod tests_wave31_agent01 {
    use super::*;

    #[test]
    fn packets() {
        assert_eq!(w31a01_pkt_1f(0x1234), [0x1f, 0x34, 0x12]);
        assert_eq!(w31a01_pkt_35(0xabcd), [0x35, 0xcd, 0xab]);
        assert_eq!(w31a01_pkt_2d(7), [0x2d, 7]);
        assert_eq!(w31a01_pkt_22(7), [0x22, 7]);
        assert_eq!(w31a01_pkt_1e(7), [0x1e, 7]);
        assert_eq!(w31a01_pkt_25(7), [0x25, 7]);
        assert_eq!(w31a01_pkt_28(7), [0x28, 7]);
        assert_eq!(w31a01_pkt_32(7), [0x32, 7]);
    }

    #[test]
    fn find_index() {
        let mut t = vec![0u32; 0x400];
        t[0] = 5;
        t[0x2f2] = 9;
        t[0x3e6] = 11;
        assert_eq!(w31a01_find_index(&t, 0, false), 0);
        assert_eq!(w31a01_find_index(&t, 5, false), 1);
        assert_eq!(w31a01_find_index(&t, 9, false), 0x2f3);
        assert_eq!(w31a01_find_index(&t, 11, false), 0);
        assert_eq!(w31a01_find_index(&t, 11, true), 0x3e7);
        assert_eq!(w31a01_find_index(&t, 77, true), 0);
    }
}
