//! Input scheduling and camera-range-aware motion, independent of rendering.
use c200_core::Control;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

pub type WriteAck = (u64, Vec<String>);

#[derive(Default)]
pub struct Edits {
    drafts: BTreeMap<String, i64>,
    pending: BTreeMap<String, (i64, Instant)>,
    in_flight: BTreeMap<String, u64>,
    ticket: u64,
}

impl Edits {
    pub fn value(&self, control: &Control) -> Option<i64> {
        self.drafts.get(&control.key).copied().or(control.value)
    }
    pub fn busy(&self) -> bool {
        !self.drafts.is_empty()
    }
    pub fn queue(&mut self, key: String, value: i64, now: Instant) {
        self.pending
            .entry(key.clone())
            .and_modify(|edit| edit.0 = value)
            .or_insert((value, now));
        self.drafts.insert(key, value);
    }
    pub fn take_due(&mut self, now: Instant) -> Option<(u64, BTreeMap<String, i64>)> {
        let keys: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, (_, first))| {
                now.saturating_duration_since(*first) >= Duration::from_millis(80)
            })
            .map(|(key, _)| key.clone())
            .collect();
        if keys.is_empty() {
            return None;
        }
        self.ticket += 1;
        let values = keys
            .into_iter()
            .map(|key| {
                let (value, _) = self.pending.remove(&key).unwrap();
                self.in_flight.insert(key.clone(), self.ticket);
                (key, value)
            })
            .collect();
        Some((self.ticket, values))
    }
    pub fn acknowledge(&mut self, acknowledgements: &[WriteAck]) -> Vec<String> {
        let mut settled = Vec::new();
        for (ticket, keys) in acknowledgements {
            for key in keys {
                if self.in_flight.get(key) == Some(ticket) {
                    self.in_flight.remove(key);
                    if !self.pending.contains_key(key) {
                        self.drafts.remove(key);
                        settled.push(key.clone());
                    }
                }
            }
        }
        settled
    }
    pub fn clear(&mut self) {
        self.pending.clear();
        self.drafts.clear();
        self.in_flight.clear();
        // Keep ticket monotonic: a delayed acknowledgement must never match a new edit.
    }
}

#[derive(Default)]
pub struct AxisMotion {
    remainder: f64,
}

impl AxisMotion {
    pub fn stop(&mut self) {
        self.remainder = 0.0;
    }
    pub fn advance(
        &mut self,
        control: &Control,
        current: i64,
        direction: f32,
        dt: f32,
        precise: bool,
    ) -> Option<i64> {
        if !control.editable()
            || control.value.is_none()
            || control.min >= control.max
            || direction.abs() < 0.001
        {
            self.stop();
            return None;
        }
        let step = control.step.max(1) as f64;
        let distance = (control.max as f64 - control.min as f64)
            * direction as f64
            * dt.clamp(0.0, 0.05) as f64
            * if precise { 0.07 } else { 0.32 };
        self.remainder += distance;
        let steps = (self.remainder / step).trunc();
        if steps == 0.0 {
            return None;
        }
        self.remainder -= steps * step;
        let max_index = ((control.max as f64 - control.min as f64) / step).floor();
        let index =
            (((current as f64 - control.min as f64) / step).round() + steps).clamp(0.0, max_index);
        let target = (control.min as f64 + index * step) as i64;
        (target != current).then_some(target)
    }
}

pub fn stick_direction(x: f32, y: f32) -> [f32; 2] {
    let length = x.hypot(y);
    if length <= 0.12 {
        return [0.0, 0.0];
    }
    let magnitude = ((length.min(1.0) - 0.12) / 0.88).clamp(0.0, 1.0);
    [x / length * magnitude, y / length * magnitude]
}

pub fn keyboard_target(
    control: &Control,
    current: i64,
    direction: i64,
    precise: bool,
) -> Option<i64> {
    if !control.editable() || control.value.is_none() || control.min >= control.max {
        return None;
    }
    let step = control.step.max(1);
    let count = if precise {
        1
    } else {
        ((control.max as i128 - control.min as i128) / step as i128 / 40).max(1)
    };
    let max_index = (control.max as i128 - control.min as i128) / step as i128;
    let index = (((current as i128 - control.min as i128) / step as i128)
        + count * direction as i128)
        .clamp(0, max_index);
    let target = (control.min as i128 + index * step as i128) as i64;
    (target != current).then_some(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn axis() -> Control {
        Control {
            min: -100,
            max: 100,
            step: 10,
            value: Some(0),
            ..c200_core::mock::mock_controls()
                .into_iter()
                .find(|c| c.key == "pan_absolute")
                .unwrap()
        }
    }
    #[test]
    fn continuous_input_is_throttled_without_waiting_for_release() {
        let mut edits = Edits::default();
        let start = Instant::now();
        for millis in [0, 20, 40, 60, 80] {
            edits.queue(
                "pan".into(),
                millis,
                start + Duration::from_millis(millis as u64),
            );
        }
        let (_, values) = edits.take_due(start + Duration::from_millis(80)).unwrap();
        assert_eq!(values["pan"], 80);
    }
    #[test]
    fn old_ack_never_clears_a_newer_in_flight_or_pending_edit() {
        let mut edits = Edits::default();
        let now = Instant::now();
        edits.queue("pan".into(), 10, now);
        let (first, _) = edits.take_due(now + Duration::from_millis(80)).unwrap();
        edits.queue("pan".into(), 20, now);
        let (second, _) = edits.take_due(now + Duration::from_millis(80)).unwrap();
        assert!(edits.acknowledge(&[(first, vec!["pan".into()])]).is_empty());
        assert!(edits.busy());
        assert_eq!(edits.acknowledge(&[(second, vec!["pan".into()])]), ["pan"]);
        assert!(!edits.busy());
    }
    #[test]
    fn motion_accumulates_sub_step_input_and_obeys_device_limits() {
        let axis = axis();
        let mut motion = AxisMotion::default();
        let mut current = 0;
        for _ in 0..100 {
            if let Some(value) = motion.advance(&axis, current, 1.0, 0.016, false) {
                current = value;
                axis.validate(value).unwrap();
            }
        }
        assert_eq!(current, 100);
        assert!(motion.advance(&axis, current, 1.0, 0.016, false).is_none());
        assert_eq!(keyboard_target(&axis, current, -1, true), Some(90));
    }
    #[test]
    fn neutral_stick_stops_and_diagonal_does_not_accelerate() {
        assert_eq!(stick_direction(0.05, 0.05), [0.0, 0.0]);
        let diagonal = stick_direction(1.0, 1.0);
        assert!((diagonal[0].hypot(diagonal[1]) - 1.0).abs() < 0.0001);
        let mut motion = AxisMotion::default();
        assert!(motion.advance(&axis(), 0, 0.0, 0.05, false).is_none());
    }
}
