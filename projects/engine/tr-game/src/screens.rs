//! 标题 / 新局 / 暂停。

use spark_core::{Color, Rect, Vec2};
use spark_input::{Input, Key, MouseBtn};
use spark_renderer::DrawList;
use spark_widget::{label, panel};

use crate::app::{Screen, TerrariaApp};

impl TerrariaApp {
    pub(crate) fn hit(input: &Input, rect: Rect) -> bool {
        let (mx, my) = input.mouse_pos();
        input.mouse_pressed(MouseBtn::Left) && rect.contains(Vec2::new(mx, my))
    }

    pub(crate) fn ui_title(&mut self, input: &Input) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        let bx = sw * 0.5 - 140.0;
        if Self::hit(input, Rect::new(bx, sh * 0.55, 280.0, 52.0))
            || input.key_pressed(Key::Enter)
            || input.key_pressed(Key::Space)
        {
            self.screen = Screen::NewGame;
        } else if Self::hit(input, Rect::new(bx, sh * 0.55 + 70.0, 280.0, 52.0)) {
            self.exit = true;
        }
    }

    pub(crate) fn ui_new_game(&mut self, input: &Input) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        if Self::hit(input, Rect::new(sw * 0.5 - 140.0, sh - 140.0, 280.0, 52.0))
            || input.key_pressed(Key::Enter)
        {
            self.start_game();
        }
        if Self::hit(input, Rect::new(40.0, sh - 80.0, 140.0, 44.0))
            || input.key_pressed(Key::Escape)
        {
            self.screen = Screen::Title;
        }
    }

    pub(crate) fn ui_pause(&mut self, input: &Input) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        let bx = sw * 0.5 - 130.0;
        if Self::hit(input, Rect::new(bx, sh * 0.42, 260.0, 48.0)) {
            self.screen = Screen::Playing;
        }
        if Self::hit(input, Rect::new(bx, sh * 0.42 + 64.0, 260.0, 48.0)) {
            self.world = None;
            self.player = None;
            self.enemies.clear();
            self.screen = Screen::Title;
        }
    }

    pub(crate) fn paint_btn(draw: &mut DrawList, x: f32, y: f32, w: f32, h: f32, text: &str) {
        let rect = Rect::new(x, y, w, h);
        draw.fill_rect(
            Rect::new(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0),
            Color::rgb(0.45, 0.70, 0.95),
        );
        draw.fill_rect(rect, Color::rgb(0.12, 0.22, 0.38));
        let size = 22.0;
        let est_w = text.chars().count() as f32 * size * 0.55;
        let tx = rect.x + (rect.w - est_w).max(0.0) * 0.5;
        let ty = rect.y + (rect.h - size) * 0.5;
        draw.text(tx, ty, size, Color::rgb(0.92, 0.96, 1.0), text);
    }

    pub(crate) fn paint_title(&self, draw: &mut DrawList) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        panel(
            draw,
            Rect::new(0.0, 0.0, sw, sh),
            Color::rgb(0.03, 0.05, 0.10),
        );
        for i in 0..40 {
            let x = ((i * 97) % 100) as f32 / 100.0 * sw;
            let y = ((i * 53) % 100) as f32 / 100.0 * sh * 0.7;
            draw.fill_rect(Rect::new(x, y, 2.0, 2.0), Color::rgba(0.8, 0.9, 1.0, 0.35));
        }
        label(
            draw,
            sw * 0.5 - 220.0,
            sh * 0.22,
            48.0,
            Color::rgb(0.85, 0.92, 1.0),
            "Terraria",
        );
        label(
            draw,
            sw * 0.5 - 90.0,
            sh * 0.22 + 56.0,
            28.0,
            Color::rgb(0.55, 0.75, 0.95),
            "Terraria",
        );
        label(
            draw,
            sw * 0.5 - 200.0,
            sh * 0.22 + 100.0,
            18.0,
            Color::rgb(0.55, 0.62, 0.75),
            "Spark 上的 Terraria 重写",
        );
        let bx = sw * 0.5 - 140.0;
        Self::paint_btn(draw, bx, sh * 0.55, 280.0, 52.0, "新游戏");
        Self::paint_btn(draw, bx, sh * 0.55 + 70.0, 280.0, 52.0, "退出");
    }

    pub(crate) fn paint_new_game(&self, draw: &mut DrawList) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        panel(
            draw,
            Rect::new(0.0, 0.0, sw, sh),
            Color::rgb(0.04, 0.06, 0.12),
        );
        label(
            draw,
            sw * 0.5 - 100.0,
            120.0,
            32.0,
            Color::rgb(0.9, 0.95, 1.0),
            "新游戏",
        );
        label(
            draw,
            sw * 0.5 - 160.0,
            180.0,
            18.0,
            Color::rgb(0.6, 0.68, 0.8),
            "进入小型测试世界",
        );
        Self::paint_btn(draw, sw * 0.5 - 140.0, sh - 140.0, 280.0, 52.0, "开始游戏");
        Self::paint_btn(draw, 40.0, sh - 80.0, 140.0, 44.0, "返回");
    }

    pub(crate) fn paint_pause(&self, draw: &mut DrawList) {
        let sw = self.screen_w;
        let sh = self.screen_h;
        panel(
            draw,
            Rect::new(0.0, 0.0, sw, sh),
            Color::rgba(0.0, 0.0, 0.0, 0.55),
        );
        label(
            draw,
            sw * 0.5 - 40.0,
            sh * 0.28,
            36.0,
            Color::rgb(1.0, 1.0, 1.0),
            "暂停",
        );
        let bx = sw * 0.5 - 130.0;
        Self::paint_btn(draw, bx, sh * 0.42, 260.0, 48.0, "继续");
        Self::paint_btn(draw, bx, sh * 0.42 + 64.0, 260.0, 48.0, "回标题");
    }
}
