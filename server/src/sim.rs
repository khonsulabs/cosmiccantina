// Create persistent people that need to eat at the same time.
// Create relationships between people that need to eat at the same time. Someitmes people eat alone,
// but sometimes they go in groups of 2, 3, or 4.
//

use chrono::Utc;
use crossbeam::atomic::AtomicCell;
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use std::collections::HashMap;

#[derive(Default)]
struct Patron {
    pub id: PatronId,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub schedules: Vec<f32>,
    pub last_time_eating: Option<f32>,
    pub preferences: HashMap<Cuisine, f32>,
    pub happiness: f32, // When they eat Cuisines they like that are high quality, they become happier
    pub hunger: f32, // When they eat a meal they don't like as much, their hunger isn't satisfied as much
    pub wealth: f32, // How much is price a factor? 1.0 - they could care less. 0.0 they will always pick the cheapest option
}
lazy_static! {
    static ref GLOBAL_ID_CELL: AtomicCell<i64> = { AtomicCell::new(0) };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
struct CantinaId(i64);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
struct PatronId(i64);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
struct DiningPartyId(i64);

impl Patron {
    pub fn random() -> Self {
        let mut rng = thread_rng();
        let id = PatronId(GLOBAL_ID_CELL.fetch_add(1));
        Self {
            id,
            name: id.0.to_string(),
            schedules: Self::random_schedules(),
            preferences: Self::random_preferences(),
            happiness: rng.gen(),
            hunger: rng.gen(),
            ..Default::default()
        }
    }

    pub fn needs_to_eat(&self, time_of_day: f32) -> bool {
        if let Some(last_time_eating) = &self.last_time_eating {
            if last_time_eating > &time_of_day {
                // It's a new day, we need to just check to see if we're past the first time we can eat
                if let Some(first_time) = self.schedules.get(0) {
                    first_time < &time_of_day
                } else {
                    // No schedules? Can't eat
                    unreachable!("All NPCs should have at least one schedule")
                }
            } else {
                // We've eaten today, find out if it's time to eat again.
                self.schedules
                    .iter()
                    .find(|t| t > &last_time_eating && t < &&time_of_day)
                    .is_some()
            }
        } else {
            true
        }
    }

    fn random_schedules() -> Vec<f32> {
        // Let's make each person eat 3 times a day, spaced out evenly
        let mut rng = thread_rng();
        let eight_hours = 8 * 60 * 60;
        let first_offset_seconds = rng.gen_range(0, eight_hours) as f32;
        let first_time = first_offset_seconds;
        let second_time = first_offset_seconds + eight_hours as f32;
        let third_time = second_time + eight_hours as f32;

        vec![first_time, second_time, third_time]
    }

    fn random_preferences() -> HashMap<Cuisine, f32> {
        let mut rng = thread_rng();
        let mut preferences = HashMap::new();
        preferences.insert(Cuisine::Beef, rng.gen());
        preferences.insert(Cuisine::Chicken, rng.gen());
        preferences.insert(Cuisine::Pork, rng.gen());
        preferences.insert(Cuisine::Shrimp, rng.gen());
        preferences.insert(Cuisine::Fish, rng.gen());
        preferences
    }

    pub fn cantina_score(&self, cantina: &Cantina) -> f32 {}
}

#[derive(Clone, Debug)]
struct DiningParty {
    pub id: DiningPartyId,
    pub patrons: Vec<PatronId>,
    pub next_decision_time: f32,
    pub state: DiningPartyState,
}

#[derive(Clone, Debug)]
enum DiningPartyState {
    DecidingWhereToGo,
    WaitingToOrder,
    Ordering {
        craving: HashMap<i64, Cuisine>,
    },
    WaitingForOrder {
        order_id: i64,
        cuisine: HashMap<i64, Cuisine>,
    },
    Eating {
        dishes: HashMap<i64, Dish>,
    },
    Leaving,
}

#[derive(Clone, Debug)]
struct Dish {
    pub cuisine: Cuisine,
    pub quality: f32,
}

#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
enum Cuisine {
    Beef,
    Chicken,
    Pork,
    Shrimp,
    Fish,
}

#[derive(Clone)]
struct Cantina {
    pub id: CantinaId,
    pub minimum_quality: f32,
    pub profit_margin: f32,
    pub worker_skill: f32,
    pub cuisines: Vec<Cuisine>,
}

struct World {
    cantinas: HashMap<CantinaId, Cantina>,
    patrons: HashMap<PatronId, Patron>,
    dining_parties: HashMap<DiningPartyId, DiningParty>,
    elapsed_seconds: f32,
    universe_time: f32,
}

impl World {
    async fn spawn_new_dining_parties(&mut self) {
        let dining_parties = self
            .patrons
            .values()
            .filter_map(|patron| {
                if patron.needs_to_eat(self.universe_time) {
                    Some(DiningParty {
                        id: DiningPartyId(GLOBAL_ID_CELL.fetch_add(1)),
                        patrons: vec![patron.id],
                        next_decision_time: self.universe_time,
                        state: DiningPartyState::DecidingWhereToGo,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for party in dining_parties {
            self.dining_parties.insert(party.id, party);
        }
    }

    async fn update_dining_parties(&mut self) {
        let mut dining_parties_to_disband = Vec::new();

        for (_, party) in self.dining_parties.iter_mut() {
            if party.next_decision_time < self.universe_time {
                continue;
            }

            match party.state {
                DiningPartyState::DecidingWhereToGo => {
                    // Pick a place to eat at.
                    let cantina_id = self.pick_cantina(&party);
                }
                _ => todo!(),
            }

            if true {
                dining_parties_to_disband.push(party.id);
            }
        }

        for entity in dining_parties_to_disband {
            self.dining_parties.remove(&entity);
        }
    }
    pub fn pick_cantina(&self, party: &DiningParty) -> CantinaId {
        // Create a score for each cantina, allowing one arbitrary person to be the person with more influence
        // this time by giving their vote twice as much weight. This just
        // Meed wealth to factor in to how much someone will want to go to a restaurant
        let mut scores = HashMap::new();

        for cantina in world.cantinas.vslues() {
            let mut effective_score = 0f32;
            for patron_id in self.patrons.iter() {
                let patron = world.patrons.get(patron_id).expect("Invalid id");
                effective_score += patron.cantina_score(cantina);
            }
        }
    }
}

pub async fn run() {
    let cantinas = vec![
        Cantina {
            id: CantinaId(0),
            minimum_quality: 0.5,
            profit_margin: 0.10,
            worker_skill: 0.75,
            cuisines: vec![Cuisine::Beef, Cuisine::Shrimp],
        },
        Cantina {
            id: CantinaId(1),
            minimum_quality: 0.4,
            profit_margin: 0.05,
            worker_skill: 0.5,
            cuisines: vec![Cuisine::Beef, Cuisine::Chicken],
        },
        Cantina {
            id: CantinaId(2),
            minimum_quality: 0.6,
            profit_margin: 0.10,
            worker_skill: 0.75,
            cuisines: vec![Cuisine::Pork, Cuisine::Fish],
        },
        Cantina {
            id: CantinaId(3),
            minimum_quality: 0.7,
            profit_margin: 0.10,
            worker_skill: 0.90,
            cuisines: vec![Cuisine::Chicken, Cuisine::Fish, Cuisine::Shrimp],
        },
    ]
    .into_iter()
    .map(|c| (c.id, c))
    .collect();

    let patrons = (0..100)
        .map(|_| {
            let p = Patron::random();
            (p.id, p)
        })
        .collect();

    let mut world = World {
        patrons,
        cantinas,
        dining_parties: HashMap::new(),
        elapsed_seconds: 0.0,
        universe_time: 0.0,
    };
    let mut last_loop_start = Utc::now();
    loop {
        let now = Utc::now();
        world.elapsed_seconds = now
            .signed_duration_since(last_loop_start)
            .num_milliseconds() as f32
            / 1_000.0;
        last_loop_start = now;
        // We don't actually care when the timestamp is issued, we just want time to pass 24 times faster than normal
        // so that each hour represents a full day. To keep it consistent across restarts we're going to just multiply
        // the current UTC timestamp by 24 to get the current universe timestamp
        world.universe_time = now.timestamp_nanos() as f32 * 24.0;

        // TODO Match them up based on relationships, if someone doesn't have a dining partner, they can wait a bit, but if they still don't have a partner after a full game-time-hour, make them dine solo and give them new relationships with other solo diners
        // For now everyone dines alone
        world.spawn_new_dining_parties().await;

        // Update each of the dining parties
        world.update_dining_parties().await;
    }
}
// POSTGRES leader election:
//   table: workers
//     id, last_seen, is_leader
//   am_i_leader(id) -> bool (does the leader switch if last_seen of the current leader is too old)
//   call it before saving and before executing
