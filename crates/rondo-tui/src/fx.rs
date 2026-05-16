use ratatui::{buffer::Buffer, layout::Rect, style::Color, Frame};
use std::time::Instant;
use tachyonfx::{Effect, Shader};

/// Force any ratatui Color into a concrete Color::Rgb (tachyonfx requires
/// Into<Color> on concrete RGB; named variants like Reset would fail HSL math).
pub fn rgb_color(c: Color) -> Color {
    let [r, g, b] = match c {
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Reset | Color::Black => [0, 0, 0],
        Color::White => [255, 255, 255],
        Color::Red => [200, 60, 60],
        Color::Green => [60, 180, 90],
        Color::Yellow => [220, 180, 60],
        Color::Blue => [60, 100, 200],
        Color::Magenta => [180, 60, 180],
        Color::Cyan => [60, 200, 220],
        Color::Gray | Color::DarkGray => [120, 120, 120],
        _ => [180, 180, 180],
    };
    Color::Rgb(r, g, b)
}

/// Identifies a live effect so callers can replace or query specific ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectId {
    StatusToast,
    DetailRefresh,
    TaskDone(i64),
    PomodoroOpen,
    QuickAddInsert,
    PageSwap,
}

struct LiveEffect {
    id: EffectId,
    effect: Effect,
    area: Rect,
}

/// In-process effect bucket. Effects are processed in spawn order and
/// rendered as overlays on top of the already-painted buffer.
pub struct FxManager {
    effects: Vec<LiveEffect>,
    last_tick: Instant,
    /// Honors RONDO_FX=0 to disable animations entirely.
    enabled: bool,
}

impl FxManager {
    pub fn new() -> Self {
        let enabled = std::env::var("RONDO_FX")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);
        Self {
            effects: Vec::new(),
            last_tick: Instant::now(),
            enabled,
        }
    }

    /// Spawn a new effect. Replaces any prior live effect with the same id.
    pub fn spawn(&mut self, id: EffectId, effect: Effect, area: Rect) {
        if !self.enabled {
            return;
        }
        // If the bucket was empty, reset the clock so dt on the first frame
        // is small. Otherwise the stale `last_tick` (set when fx went idle)
        // could be seconds old, completing the new effect in one frame.
        if self.effects.is_empty() {
            self.last_tick = Instant::now();
        }
        self.effects.retain(|e| e.id != id);
        self.effects.push(LiveEffect { id, effect, area });
    }

    /// True when at least one effect is still running.
    pub fn any_running(&self) -> bool {
        !self.effects.is_empty()
    }

    /// Apply all live effects to the frame buffer, dropping the ones that finished.
    pub fn tick_and_render(&mut self, f: &mut Frame<'_>) {
        if self.effects.is_empty() {
            self.last_tick = Instant::now();
            return;
        }
        // Clamp dt at 64ms so an occasional slow frame can't fast-forward
        // every active effect to completion.
        let raw = self.last_tick.elapsed();
        self.last_tick = Instant::now();
        let clamped_ms = raw.as_millis().min(64) as u32;
        let dt = tachyonfx::Duration::from_millis(clamped_ms);
        let frame_area = f.area();
        let buf: &mut Buffer = f.buffer_mut();
        self.effects.retain_mut(|live| {
            let clipped = live.area.intersection(frame_area);
            if clipped.width == 0 || clipped.height == 0 {
                return false;
            }
            live.effect.process(dt, buf, clipped);
            !live.effect.done()
        });
    }

    /// Helper for tests / config UI.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for FxManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-built ergonomic constructors so call sites stay readable.
pub mod presets {
    use super::rgb_color;
    use ratatui::style::Color;
    use tachyonfx::{fx, Effect, EffectTimer, Interpolation, Motion};

    pub fn status_toast(accent: Color, muted: Color) -> Effect {
        fx::sequence(&[
            fx::fade_from_fg(rgb_color(accent), EffectTimer::from_ms(120, Interpolation::QuadIn)),
            fx::sleep(EffectTimer::from_ms(900, Interpolation::Linear)),
            fx::fade_to_fg(rgb_color(muted), EffectTimer::from_ms(220, Interpolation::QuadOut)),
        ])
    }

    pub fn detail_refresh(accent: Color) -> Effect {
        // Single coalesce: assembles cells from blank → fully-painted in one
        // pass. Cleaner than dissolve→fade because dissolve fully clears the
        // panel before sequence advances; some frames would land between
        // effects and look black/partial.
        fx::parallel(&[
            fx::coalesce(EffectTimer::from_ms(220, Interpolation::CubicOut)),
            fx::fade_from_fg(rgb_color(accent), EffectTimer::from_ms(180, Interpolation::QuadOut)),
        ])
    }

    pub fn task_done_sweep(muted: Color) -> Effect {
        fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            rgb_color(muted),
            EffectTimer::from_ms(260, Interpolation::CubicOut),
        )
    }

    pub fn pomodoro_open(accent: Color) -> Effect {
        fx::fade_from_fg(rgb_color(accent), EffectTimer::from_ms(280, Interpolation::QuadIn))
    }

    pub fn quick_add_slide(bg: Color) -> Effect {
        fx::slide_in(
            Motion::UpToDown,
            6,
            0,
            rgb_color(bg),
            EffectTimer::from_ms(220, Interpolation::QuadOut),
        )
    }

    pub fn page_swap(bg: Color) -> Effect {
        fx::sweep_in(
            Motion::LeftToRight,
            12,
            0,
            rgb_color(bg),
            EffectTimer::from_ms(220, Interpolation::QuadOut),
        )
    }
}
