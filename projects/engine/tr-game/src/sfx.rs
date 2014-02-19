//! 玩法音效：优先 `Content/Sounds` XNB，失败时回退程序化短音。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use spark_audio::{AudioBus, PcmAudio, Tone};

use crate::xnb::{PcmSound, decode_sound_file};

static DIG_ROT: AtomicUsize = AtomicUsize::new(0);
static TINK_ROT: AtomicUsize = AtomicUsize::new(0);
static HIT_ROT: AtomicUsize = AtomicUsize::new(0);

/// 预解码的Content 音效库。
#[derive(Debug, Default)]
pub struct SfxBank {
    dig: Vec<PcmAudio>,
    tink: Vec<PcmAudio>,
    hit: Vec<PcmAudio>,
    grab: Option<PcmAudio>,
    swing: Option<PcmAudio>,
    chat: Option<PcmAudio>,
    coins: Option<PcmAudio>,
    ready: bool,
}

impl SfxBank {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从安装根加载常用音效。缺文件不致命，对应事件仍走程序化音。
    pub fn ensure(&mut self, install: &Path) {
        if self.ready {
            return;
        }
        self.ready = true;
        let dir = install.join("Content").join("Sounds");
        if !dir.is_dir() {
            tracing::warn!(path = %dir.display(), "缺少 Content/Sounds，音效回退程序化");
            return;
        }

        self.dig = load_variants(&dir, "Dig", 3);
        self.tink = load_variants(&dir, "Tink", 3);
        self.hit = load_variants(&dir, "Player_Hit", 3);
        self.grab = load_one(&dir, "Grab.xnb");
        self.swing = load_one(&dir, "Item_1.xnb");
        self.chat = load_one(&dir, "Chat.xnb");
        self.coins = load_one(&dir, "Coins.xnb").or_else(|| load_one(&dir, "Coin_0.xnb"));

        tracing::info!(
            dig = self.dig.len(),
            tink = self.tink.len(),
            hit = self.hit.len(),
            grab = self.grab.is_some(),
            swing = self.swing.is_some(),
            chat = self.chat.is_some(),
            coins = self.coins.is_some(),
            "Content 音效已加载"
        );
    }

    fn play(&self, audio: &AudioBus, pcm: &PcmAudio) {
        if let Err(e) = audio.play_pcm(pcm) {
            tracing::debug!(error = %e, "音效播放失败");
        }
    }

    fn play_rot(&self, audio: &AudioBus, list: &[PcmAudio], rot: &AtomicUsize, fallback: Tone) {
        if list.is_empty() {
            audio.play_tone(fallback);
            return;
        }
        let i = rot.fetch_add(1, Ordering::Relaxed) % list.len();
        self.play(audio, &list[i]);
    }
}

fn load_one(dir: &Path, name: &str) -> Option<PcmAudio> {
    let path = dir.join(name);
    match decode_sound_file(&path) {
        Ok(s) => Some(to_pcm(s)),
        Err(e) => {
            tracing::warn!(file = name, %e, "音效解码失败");
            None
        }
    }
}

fn load_variants(dir: &Path, prefix: &str, n: u32) -> Vec<PcmAudio> {
    let mut out = Vec::new();
    for i in 0..n {
        let name = format!("{prefix}_{i}.xnb");
        if let Some(pcm) = load_one(dir, &name) {
            out.push(pcm);
        }
    }
    out
}

fn to_pcm(s: PcmSound) -> PcmAudio {
    PcmAudio {
        sample_rate: s.sample_rate,
        channels: s.channels,
        samples: s.samples,
    }
}

pub fn dig_chip(bank: &SfxBank, audio: &AudioBus) {
    bank.play_rot(audio, &bank.dig, &DIG_ROT, Tone::new(220.0, 35, 0.28));
}

pub fn dig_break(bank: &SfxBank, audio: &AudioBus) {
    // 打碎优先用 Dig；石头感更强时已有 Tink 变体可扩展。
    bank.play_rot(audio, &bank.dig, &DIG_ROT, Tone::new(140.0, 70, 0.4));
}

pub fn dig_tink(bank: &SfxBank, audio: &AudioBus) {
    if bank.tink.is_empty() {
        dig_chip(bank, audio);
        return;
    }
    bank.play_rot(audio, &bank.tink, &TINK_ROT, Tone::new(320.0, 40, 0.3));
}

pub fn place(bank: &SfxBank, audio: &AudioBus) {
    if let Some(pcm) = bank.dig.first() {
        bank.play(audio, pcm);
    } else {
        audio.play_tone(Tone::new(420.0, 45, 0.32));
    }
}

pub fn melee_hit(bank: &SfxBank, audio: &AudioBus) {
    if let Some(pcm) = &bank.swing {
        bank.play(audio, pcm);
    } else {
        audio.play_tone(Tone::new(90.0, 55, 0.45));
    }
}

pub fn player_hurt(bank: &SfxBank, audio: &AudioBus) {
    bank.play_rot(audio, &bank.hit, &HIT_ROT, Tone::new(110.0, 80, 0.4));
}

pub fn craft_ok(bank: &SfxBank, audio: &AudioBus) {
    let _ = bank;
    audio.play_tone(Tone::new(660.0, 50, 0.3));
}

pub fn pickup(bank: &SfxBank, audio: &AudioBus) {
    if let Some(pcm) = &bank.grab {
        bank.play(audio, pcm);
    } else {
        audio.play_tone(Tone::new(880.0, 30, 0.22));
    }
}

pub fn chat(bank: &SfxBank, audio: &AudioBus) {
    if let Some(pcm) = &bank.chat {
        bank.play(audio, pcm);
    } else {
        audio.play_tone(Tone::new(520.0, 40, 0.28));
    }
}

pub fn coins(bank: &SfxBank, audio: &AudioBus) {
    if let Some(pcm) = &bank.coins {
        bank.play(audio, pcm);
    } else {
        audio.play_tone(Tone::new(980.0, 45, 0.28));
    }
}
