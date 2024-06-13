use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, KeyMapChar, MotionAction},
    InputStatus,
};
use glam::{vec2, Vec2};

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

// scale * 10 (because atomic floats don't exist)
static SCALE_FACTOR: AtomicU32 = AtomicU32::new(35);
pub fn scale_factor() -> f32 {
    SCALE_FACTOR.load(Ordering::SeqCst) as f32 * 0.1
}
pub fn set_scale_factor(factor: f32) {
    let factor = (factor * 10.0) as u32;
    if factor > 0 {
        SCALE_FACTOR.store(factor, Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PtrButton {
    Primary = 0,
    Secondary = 1,
    Middle = 2,
    Extra1 = 3,
    Extra2 = 4,
}

#[derive(Clone, Copy, Debug)]
pub struct Ptr {
    pub pos: Vec2,
    pub id: u32,
}

enum PtrChange {
    New(Ptr),
    Rm(Ptr),
    Move(Ptr),
}
fn pointers_diff<'a>(prev: &'a [Ptr], new: &'a [Ptr]) -> impl Iterator<Item = PtrChange> + 'a {
    let new_iter = new
        .iter()
        .filter(|ptr| !prev.iter().any(|pptr| pptr.id == ptr.id))
        .map(|ptr| PtrChange::New(*ptr));
    let rm_iter = prev
        .iter()
        .filter(|ptr| !new.iter().any(|nptr| nptr.id == ptr.id))
        .map(|ptr| PtrChange::Rm(*ptr));
    let mv_iter = new
        .iter()
        .filter(|ptr| {
            prev.iter()
                .any(|pptr| pptr.id == ptr.id && pptr.pos != ptr.pos)
        })
        .map(|ptr| PtrChange::Move(*ptr));
    new_iter.chain(mv_iter).chain(rm_iter)
}

#[derive(Clone, Copy, Debug)]
pub struct Zoom {
    pub start_dist: f32,
    pub prev_dist: f32,
    pub anchor: Vec2,
}

#[derive(Clone, Debug)]
pub enum TouchEvent {
    Text(String),
    KeyBackspace { pressed: bool },
    PtrMoved(Vec2),
    PtrPressed(PtrButton, Vec2),
    PtrReleased(PtrButton, Vec2),
    PtrLeft,
    Zoom(f32, Vec2),
    Scroll(Vec2),
}

#[derive(Clone, Debug)]
pub struct TouchTranslater {
    prev_pointers: Vec<Ptr>,
    scale_factor: f32,
    ignore_release: bool,
    last_press_time: SystemTime,
    last_pos: Vec2,
    press_pos: Option<Vec2>,
    holding: bool,
    pointer_count: u32,
    pointers: Vec<Option<Ptr>>,
    zoom: Option<Zoom>,
    wants_zoom: bool,
}
impl Default for TouchTranslater {
    fn default() -> Self {
        Self {
            prev_pointers: Vec::new(),
            scale_factor: 1.0,
            ignore_release: false,
            last_press_time: SystemTime::UNIX_EPOCH,
            last_pos: Vec2::ZERO,
            press_pos: None,
            holding: false,
            pointer_count: 0,
            pointers: vec![],
            zoom: None,
            wants_zoom: false,
        }
    }
}
impl TouchTranslater {
    pub fn set_scale_factor(&mut self, f: f32) {
        self.scale_factor = f;
    }

    pub fn update_pointers(&mut self, ptrs: Vec<Ptr>, mut out: impl FnMut(TouchEvent)) {
        let mut temp_prev_pointers = Vec::new();
        std::mem::swap(&mut temp_prev_pointers, &mut self.prev_pointers);

        for change in pointers_diff(&temp_prev_pointers, &ptrs) {
            match change {
                PtrChange::New(ptr) => self.phase_start(ptr.id as usize, ptr.pos, &mut out),
                PtrChange::Rm(ptr) => self.phase_end(ptr.id as usize, ptr.pos, &mut out),
                PtrChange::Move(ptr) => self.phase_move(ptr.id as usize, ptr.pos, &mut out),
            }
        }
        std::mem::swap(&mut temp_prev_pointers, &mut self.prev_pointers);
        self.prev_pointers = ptrs;
    }

    pub fn update(&mut self, mut out: impl FnMut(TouchEvent)) {
        if self.holding
            && SystemTime::now()
                .duration_since(self.last_press_time)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                > 500
        {
            out(TouchEvent::PtrPressed(
                PtrButton::Secondary,
                self.last_pos / self.scale_factor,
            ));
            out(TouchEvent::PtrReleased(
                PtrButton::Secondary,
                self.last_pos / self.scale_factor,
            ));
            self.ignore_release = true;
            self.holding = false;
        }
    }

    fn phase_start(&mut self, idx: usize, pos: Vec2, mut out: impl FnMut(TouchEvent)) {
        self.pointer_count += 1;
        if idx >= self.pointers.len() {
            self.pointers.resize(idx + 1, None);
        }
        self.pointers[idx] = Some(Ptr {
            pos,
            id: idx as u32,
        });

        if self.pointer_count == 2 && self.wants_zoom {
            self.press_pos = None;
            self.ignore_release = true;
            self.holding = false;

            out(TouchEvent::PtrLeft);

            let mut pointers = self.pointers.iter().cloned().flatten();
            let [a, b] = [pointers.next().unwrap(), pointers.next().unwrap()];
            let dist = a.pos.distance_squared(b.pos);
            let (min, max) = (a.pos.min(b.pos), a.pos.max(b.pos));
            let anchor = min + (max - min) * 0.5;
            self.zoom = Some(Zoom {
                start_dist: dist,
                prev_dist: dist,
                anchor,
            });
        } else {
            out(TouchEvent::PtrMoved(pos / self.scale_factor));
            out(TouchEvent::PtrPressed(
                PtrButton::Primary,
                pos / self.scale_factor,
            ));

            self.last_pos = pos;
            self.last_press_time = SystemTime::now();
            self.press_pos = Some(pos);
            self.holding = true;
            self.ignore_release = false;
        }
    }

    fn phase_move(&mut self, idx: usize, pos: Vec2, mut out: impl FnMut(TouchEvent)) {
        self.last_pos = pos;
        if self.pointer_count == 1 {
            out(TouchEvent::PtrMoved(pos / self.scale_factor));
        }

        if let Some(press_pos) = self.press_pos {
            let press_dist = press_pos.distance_squared(pos).abs();
            if press_dist >= 50.0 / scale_factor() * 50.0 / scale_factor() {
                self.holding = false;
                self.press_pos = None;
            }
        }
        if let Some(Some(ptr)) = self.pointers.get_mut(idx) {
            ptr.pos = pos;
        }
        if let Some(zoom) = &mut self.zoom {
            let mut pointers = self.pointers.iter().cloned().flatten();
            let [a, b] = [pointers.next().unwrap(), pointers.next().unwrap()];
            let dist = a.pos.distance_squared(b.pos);
            if dist != zoom.start_dist {
                let delta = (dist - zoom.prev_dist) * 0.0003;
                out(TouchEvent::Zoom(delta, pos / self.scale_factor));
            }
            zoom.prev_dist = dist;
        }
    }

    fn phase_end(&mut self, idx: usize, pos: Vec2, mut out: impl FnMut(TouchEvent)) {
        out(TouchEvent::PtrReleased(
            PtrButton::Primary,
            pos / self.scale_factor,
        ));

        self.press_pos = None;
        self.holding = false;
        if self.pointer_count == 1 {
            out(TouchEvent::PtrLeft);
        }

        if self.pointer_count == 2 {
            self.zoom = None;
        }

        if idx < self.pointers.len() {
            self.pointers[idx] = None;
            self.pointer_count -= 1;
        }
    }
}

pub fn translate_input_event(
    event: &InputEvent,
    translater: &mut TouchTranslater,
    mut out: impl FnMut(TouchEvent),
) -> InputStatus {
    match event {
        InputEvent::KeyEvent(key_event) => {
            let mut new_event = None;
            let combined_key_char =
                character_map_and_combine_key(key_event, &mut None, &mut new_event);
            match combined_key_char {
                Some(KeyMapChar::Unicode(ch)) | Some(KeyMapChar::CombiningAccent(ch)) => {
                    out(TouchEvent::Text(ch.to_string()));
                }
                None => {}
                other => log::warn!("unrecognized key_char: {other:?}"),
            }
            if let Some(event) = new_event {
                out(event);
            }
        }
        InputEvent::MotionEvent(motion_event) => {
            let pointers: Vec<_> = motion_event.pointers().collect();
            let mut pointers: Vec<_> = pointers
                .into_iter()
                .map(|ptr| Ptr {
                    id: ptr.pointer_id() as u32,
                    pos: vec2(ptr.x(), ptr.y()),
                })
                .collect();
            if matches!(
                motion_event.action(),
                MotionAction::Up | MotionAction::Cancel | MotionAction::PointerUp
            ) {
                let ptr = motion_event.pointer_at_index(motion_event.pointer_index());
                let idx = pointers
                    .iter()
                    .enumerate()
                    .find(|(_, ptr2)| ptr2.id == ptr.pointer_id() as u32)
                    .unwrap()
                    .0;
                _ = pointers.remove(idx);
            }
            translater.update_pointers(pointers, out);
        }
        InputEvent::TextEvent(text_state) => {
            log::info!("Android set text input to {text_state:?}")
        }
        _ => return InputStatus::Unhandled,
    }
    InputStatus::Handled
}

/// Tries to map the `key_event` to a `KeyMapChar` containing a unicode character or dead key accent
fn character_map_and_combine_key(
    key_event: &KeyEvent,
    combining_accent: &mut Option<char>,
    app_event: &mut Option<TouchEvent>,
) -> Option<KeyMapChar> {
    let device_id = key_event.device_id();

    use android_activity::input::Keycode;
    if key_event.key_code() == Keycode::Del {
        match key_event.action() {
            KeyAction::Up => *app_event = Some(TouchEvent::KeyBackspace { pressed: false }),
            KeyAction::Down => *app_event = Some(TouchEvent::KeyBackspace { pressed: true }),
            _ => {}
        }
        return None;
    }

    let key_map = match crate::android().device_key_character_map(device_id) {
        Ok(key_map) => key_map,
        Err(err) => {
            log::warn!("Failed to look up `KeyCharacterMap` for device {device_id}: {err:?}");
            return None;
        }
    };

    match key_map.get(key_event.key_code(), key_event.meta_state()) {
        Ok(KeyMapChar::Unicode(unicode)) => {
            // Only do dead key combining on key down
            if key_event.action() == KeyAction::Down {
                let combined_unicode = if let Some(accent) = combining_accent {
                    match key_map.get_dead_char(*accent, unicode) {
                        Ok(Some(key)) => {
                            log::warn!("KeyEvent: Combined '{unicode}' with accent '{accent}' to give '{key}'");
                            Some(key)
                        }
                        Ok(None) => None,
                        Err(err) => {
                            log::warn!("KeyEvent: Failed to combine 'dead key' accent '{accent}' with '{unicode}': {err:?}");
                            None
                        }
                    }
                } else {
                    Some(unicode)
                };
                *combining_accent = None;
                combined_unicode.map(KeyMapChar::Unicode)
            } else {
                None
            }
        }
        Ok(KeyMapChar::CombiningAccent(accent)) => {
            if key_event.action() == KeyAction::Down {
                *combining_accent = Some(accent);
            }
            Some(KeyMapChar::CombiningAccent(accent))
        }
        Ok(KeyMapChar::None) => {
            // Leave any combining_accent state in tact (seems to match how other
            // Android apps work)
            log::warn!("KeyEvent: Pressed non-unicode key");
            None
        }
        Err(err) => {
            log::warn!("KeyEvent: Failed to get key map character: {err:?}");
            *combining_accent = None;
            None
        }
    }
}
