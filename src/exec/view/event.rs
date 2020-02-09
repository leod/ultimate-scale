use nalgebra as na;

use crate::exec::view::Config;
use crate::exec::{Blip, BlipDieMode, BlipIndex, BlipStatus, Exec};

pub enum TransduceEvent {
    BlipDeath {
        blip_index: BlipIndex,
        time: f32,
    },
    BlipSliver {
        blip_index: BlipIndex,
        start_time: f32,
        duration: f32,
    },
}

impl TransduceEvent {
    pub fn num_particles(&self, _distance: f32) -> usize {
        match self {
            TransduceEvent::BlipDeath { .. } => 1000,
            TransduceEvent::BlipSliver { duration, .. } => (600.0 * duration) as usize,
        }
    }
}

const MAX_TRANSDUCE_DISTANCE_SQ: f32 = 10000.0;
const MAX_CLOSE_DISTANCE: f32 = 10.0;

pub fn iter_nearby_blips<'a>(
    exec: &'a Exec,
    eye_pos: &'a na::Point3<f32>,
) -> impl Iterator<Item = (BlipIndex, f32, &'a Blip)> {
    exec.blips().iter().filter_map(move |(blip_index, blip)| {
        let blip_pos: na::Point3<f32> = na::convert(blip.pos);
        let delta = blip_pos - eye_pos;
        let distance_sq = delta.norm_squared();

        if distance_sq > MAX_TRANSDUCE_DISTANCE_SQ {
            None
        } else {
            Some((blip_index, distance_sq.sqrt(), blip))
        }
    })
}

pub fn iter_transduce_events<'a>(
    exec: &'a Exec,
    eye_pos: &'a na::Point3<f32>,
) -> impl Iterator<Item = (f32, TransduceEvent)> + 'a {
    let death = iter_nearby_blips(exec, eye_pos).filter_map(|(blip_index, distance, blip)| {
        blip.status
            .die_mode()
            .filter(|die_mode| *die_mode != BlipDieMode::PressButton)
            .map(|die_mode| {
                let die_time = match die_mode {
                    BlipDieMode::PopEarly => 0.3,
                    _ => 0.8,
                };

                (
                    distance,
                    TransduceEvent::BlipDeath {
                        blip_index,
                        time: die_time,
                    },
                )
            })
    });

    let sliver = iter_nearby_blips(exec, eye_pos).filter_map(|(blip_index, distance, blip)| {
        let (start_time, duration) = match blip.status {
            BlipStatus::Spawning(_) => Some((0.5, 0.5)),
            BlipStatus::Existing => None,
            BlipStatus::LiveToDie(_, BlipDieMode::PressButton) => Some((0.65, 0.45)),
            BlipStatus::LiveToDie(_, _) => Some((0.5, 0.3)),
            BlipStatus::Dying(BlipDieMode::PressButton) => Some((0.65, 0.35)),
            BlipStatus::Dying(_) => None,
        }?;

        Some((
            distance,
            TransduceEvent::BlipSliver {
                blip_index,
                start_time,
                duration,
            },
        ))
    });

    death.chain(sliver)
}

pub fn compute_transduce_events(
    exec: &Exec,
    config: &Config,
    eye_pos: &na::Point3<f32>,
    events: &mut Vec<(f32, TransduceEvent)>,
    particle_budget: &mut Vec<f32>,
) {
    events.clear();
    events.extend(iter_transduce_events(exec, eye_pos));

    particle_budget.clear();
    particle_budget.reserve(events.len());

    let num_particles: usize = events
        .iter()
        .map(|(distance, event)| event.num_particles(*distance))
        .sum();

    // This code is so bad that I got a cold for a week after writing it.
    if num_particles > config.particle_budget_per_tick {
        events.sort_unstable_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

        let close_particle_budget = config.close_particle_budget_per_tick();
        assert!(
            close_particle_budget > 0 && close_particle_budget < config.particle_budget_per_tick
        );

        let mut num_spawned: usize = 0;
        let mut i = 0;
        while num_spawned < close_particle_budget && events[i].0 < MAX_CLOSE_DISTANCE {
            particle_budget.push(1.0);

            num_spawned += events[i].1.num_particles(events[i].0);
            i += 1;
        }

        let remaining_budget = config.particle_budget_per_tick - num_spawned;
        let remaining_particles = num_particles - num_spawned;
        let fraction = remaining_budget as f32 / remaining_particles as f32;

        /*log::info!(
            "num_particles {} num_spawned {} fraction {}",
            num_particles,
            num_spawned,
            fraction
        );*/

        while num_spawned < config.particle_budget_per_tick {
            particle_budget.push(fraction);

            num_spawned +=
                (events[i].1.num_particles(events[i].0) as f32 * fraction).ceil() as usize;
            i += 1;
        }

        while i < events.len() {
            particle_budget.push(0.0);
            i += 1;
        }
    } else {
        particle_budget.extend(std::iter::repeat(1.0).take(events.len()));
    }

    assert!(particle_budget.len() == events.len());
}
