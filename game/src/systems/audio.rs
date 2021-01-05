use crate::resources;
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::io::BufReader;

pub enum AudioSysMsg {
    _SetMasterVolume(f32),
    _SetMusicVolume(f32),
    _SetEffectsVolume(f32),

    _PlayMusic,
    _StopMusic,

    _PlayEffect(usize),
    _StopEffect(usize),

    _Kill,
}

pub struct AudioSystem {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,

    music: rodio::Sink,
    _sound_effects: Vec<rodio::Sink>,
    is_kill: bool,
}

impl AudioSystem {
    fn init() -> Self {
        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

        let music_source =
            rodio::Decoder::new(BufReader::new(std::io::Cursor::new(resources::MUSIC)))
                .unwrap()
                .speed(0.5)
                .repeat_infinite();

        let music_sink = rodio::Sink::try_new(&stream_handle).expect("could not create music sink");
        music_sink.append(music_source);

        Self {
            _stream,
            _stream_handle: stream_handle,
            music: music_sink,
            _sound_effects: vec![],
            is_kill: false,
        }
    }

    pub fn start() -> crossbeam_channel::Sender<AudioSysMsg> {
        let (tx, rx) = crossbeam_channel::bounded(8);

        std::thread::spawn(move || {
            let mut sys = Self::init();

            while !sys.is_kill {
                while !rx.is_empty() {
                    sys.handle_msg(
                        rx.recv()
                            .expect("failed to read msg? I didn't think this was possible!"),
                    );
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        tx
    }

    fn handle_msg(&mut self, msg: AudioSysMsg) {
        match msg {
            AudioSysMsg::_SetMasterVolume(_) => {}
            AudioSysMsg::_SetMusicVolume(_) => {}
            AudioSysMsg::_SetEffectsVolume(_) => {}
            AudioSysMsg::_PlayMusic => {}
            AudioSysMsg::_StopMusic => {}
            AudioSysMsg::_PlayEffect(_) => {}
            AudioSysMsg::_StopEffect(_) => {}
            AudioSysMsg::_Kill => {
                self.is_kill = true;
            }
        };
    }
}
