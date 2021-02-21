// mod rodio_crud;

use crate::resources;
// use crate::systems::audio::rodio_crud::Buffered;
use rodio::source::Buffered;
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::io::BufReader;
use std::sync::Arc;

pub enum AudioSysMsg {
    _SetMasterVolume(f32),
    _SetMusicVolume(f32),
    _SetEffectsVolume(f32),

    _PlayMusic,
    _StopMusic,

    PlayEffect(usize),
    _StopEffect(usize),

    _Kill,
}

pub struct AudioSystem {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    music: rodio::Sink,
    // xyz: Buffered<Box<dyn Source<Item = f32>>>,
    // sfx_src: Vec<Box<rodio::Decoder<u8>>>,
    // sound_effects: Vec<rodio::Sink>,
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

        // let src = Box::new(
        //     rodio::Decoder::new(BufReader::new(std::io::Cursor::new(resources::MUSIC)))
        //         .unwrap()
        //         .convert_samples(),
        // );
        // let bs = rodio_crud::buffered(src);

        Self {
            _stream,
            stream_handle,
            music: music_sink,
            // xyz: bs,
            // sfx_src: vec![Box::new(srx2)],
            // sound_effects: vec![sfx1_sink],
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
            AudioSysMsg::PlayEffect(id) => {
                let src = rodio::Decoder::new(BufReader::new(std::io::Cursor::new(
                    resources::EFFECT_BLIP,
                )))
                .unwrap(); // TODO: find out if a buffered source is useful at all

                let sink = rodio::Sink::try_new(&self.stream_handle)
                    .expect("could not create effect sink");
                sink.set_volume(0.1);
                sink.append(src);
                sink.detach();
            }
            AudioSysMsg::_StopEffect(_) => {}
            AudioSysMsg::_Kill => {
                self.is_kill = true;
            }
        };
    }
}
