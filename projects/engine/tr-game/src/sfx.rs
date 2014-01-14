//! 竖切提示音：把玩法事件映射到 `spark-audio` 程序化短音。

use spark_audio::{AudioBus, Tone};

pub fn dig_chip(audio: &AudioBus) {
    audio.play_tone(Tone::new(220.0, 35, 0.28));
}

pub fn dig_break(audio: &AudioBus) {
    audio.play_tone(Tone::new(140.0, 70, 0.4));
}

pub fn place(audio: &AudioBus) {
    audio.play_tone(Tone::new(420.0, 45, 0.32));
}

pub fn melee_hit(audio: &AudioBus) {
    audio.play_tone(Tone::new(90.0, 55, 0.45));
}

pub fn warp(audio: &AudioBus) {
    audio.play_tone(Tone::new(520.0, 90, 0.35));
    audio.play_tone(Tone::new(780.0, 120, 0.28));
}

pub fn craft_ok(audio: &AudioBus) {
    audio.play_tone(Tone::new(660.0, 50, 0.3));
}

pub fn pickup(audio: &AudioBus) {
    audio.play_tone(Tone::new(880.0, 30, 0.22));
}

pub fn objective(audio: &AudioBus) {
    audio.play_tone(Tone::new(523.0, 60, 0.28));
    audio.play_tone(Tone::new(659.0, 80, 0.28));
}
