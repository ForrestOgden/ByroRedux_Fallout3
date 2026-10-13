//! M47.2 — Papyrus update registrations: `RegisterForUpdate`,
//! `RegisterForSingleUpdate`, their `GameTime` twins and the `Unregister`
//! pair, delivering `OnUpdate` / `OnUpdateGameTime` to the script that
//! registered — and Fallout 4's timers, `StartTimer(afInterval, aiTimerID)`
//! / `StartTimerGameTime` and their `Cancel` pair, delivering
//! `OnTimer(aiTimerID)` / `OnTimerGameTime(aiTimerID)`.
//!
//! The rules follow the CK's *RegisterForUpdate - Form* family:
//! a periodic registration replaces the script's earlier periodic one, a
//! single one its earlier single one, and the two do not interfere;
//! `UnregisterForUpdate` cancels both; a quest's registrations end when the
//! quest stops. Real-time intervals count the frames' `dt` (the simulation
//! does not advance in menu mode); game-time intervals count game hours on
//! the absolute clock the host reports, so sleeping and waiting deliver the
//! update they pass.
//!
//! The FO4 CK's *StartTimer - ScriptObject* family: a timer is one-shot; its
//! ID is unique per script, and real-time and game-time IDs do not conflict;
//! starting a timer whose ID is already ticking resets it; a game-time
//! interval under 0.033 hours (2 minutes) is rounded up to it.
//!
//! Registrations are per script instance — the object the script is
//! attached to and the script — and the update goes to that script only.
//! The Papyrus VM check accepts a registration only when the script's
//! handler runs whole, so a delivery never meets an unchecked body. A
//! reference's update waits while the reference is unloaded and is
//! delivered when it loads again.

use super::*;

/// Which clock an update registration counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub enum UpdateClock {
    /// Seconds of simulation (`OnUpdate`).
    RealTime,
    /// Game hours (`OnUpdateGameTime`).
    GameTime,
}

impl UpdateClock {
    fn event(self, timer: bool) -> &'static str {
        match (self, timer) {
            (UpdateClock::RealTime, false) => "OnUpdate",
            (UpdateClock::GameTime, false) => "OnUpdateGameTime",
            (UpdateClock::RealTime, true) => "OnTimer",
            (UpdateClock::GameTime, true) => "OnTimerGameTime",
        }
    }
}

/// FO4 *StartTimerGameTime*: "Any value smaller then 0.033 (2 minutes) will
/// be rounded up to that value."
const MIN_GAME_TIMER_HOURS: f32 = 0.033;

/// One script instance's registration.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct UpdateRegistration {
    /// The object the script is attached to: a quest or a reference.
    pub owner: u32,
    pub script: String,
    pub clock: UpdateClock,
    /// `RegisterForUpdate` (periodic) rather than `RegisterForSingleUpdate`.
    pub recurring: bool,
    /// A Fallout 4 timer's ID (`StartTimer`); `None` for an update
    /// registration.
    pub timer: Option<i32>,
    /// Seconds or game hours between updates.
    pub interval: f32,
    /// When the next update is due, on `clock`; `None` until the game clock
    /// is first reported, when it is armed from there.
    pub due: Option<f64>,
}

/// A registration change a script run asks for, applied after the quest
/// guards drop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateRequest {
    Register {
        clock: UpdateClock,
        interval: f32,
        recurring: bool,
    },
    Unregister(UpdateClock),
    /// `StartTimer` / `StartTimerGameTime`.
    StartTimer { clock: UpdateClock, interval: f32, id: i32 },
    /// `CancelTimer` / `CancelTimerGameTime`.
    CancelTimer { clock: UpdateClock, id: i32 },
}

impl UpdateRequest {
    /// The request a `Self.<name>(args)` call makes, by lowercase name, from
    /// its numeric arguments (`None` for one that is not a number).
    pub(super) fn of_call(name: &str, args: &[Option<f32>]) -> Option<Self> {
        let interval = || args.first().copied().flatten();
        // A timer ID: the given argument, or the default 0.
        let id = |index: usize| -> Option<i32> {
            match args.get(index) {
                None => Some(0),
                Some(value) => value.map(|value| value as i32),
            }
        };
        let register = |clock, recurring| {
            Some(UpdateRequest::Register {
                clock,
                interval: interval()?,
                recurring,
            })
        };
        let start = |clock| {
            Some(UpdateRequest::StartTimer {
                clock,
                interval: interval()?,
                id: id(1)?,
            })
        };
        let cancel = |clock| Some(UpdateRequest::CancelTimer { clock, id: id(0)? });
        match name {
            "registerforupdate" => register(UpdateClock::RealTime, true),
            "registerforsingleupdate" => register(UpdateClock::RealTime, false),
            "registerforupdategametime" => register(UpdateClock::GameTime, true),
            "registerforsingleupdategametime" => register(UpdateClock::GameTime, false),
            "unregisterforupdate" => Some(UpdateRequest::Unregister(UpdateClock::RealTime)),
            "unregisterforupdategametime" => Some(UpdateRequest::Unregister(UpdateClock::GameTime)),
            "starttimer" => start(UpdateClock::RealTime),
            "starttimergametime" => start(UpdateClock::GameTime),
            "canceltimer" => cancel(UpdateClock::RealTime),
            "canceltimergametime" => cancel(UpdateClock::GameTime),
            _ => None,
        }
    }

    /// The handler a registration delivers to: what the check must accept
    /// (`""` for a cancellation).
    pub(super) fn handler(name: &str) -> Option<&'static str> {
        match name {
            "registerforupdate" | "registerforsingleupdate" => Some("OnUpdate"),
            "registerforupdategametime" | "registerforsingleupdategametime" => Some("OnUpdateGameTime"),
            "starttimer" => Some("OnTimer"),
            "starttimergametime" => Some("OnTimerGameTime"),
            "unregisterforupdate" | "unregisterforupdategametime" | "canceltimer" | "canceltimergametime" => {
                Some("")
            }
            _ => None,
        }
    }

    /// How many numeric arguments the call takes, at most.
    pub(super) fn max_args(name: &str) -> usize {
        match name {
            "starttimer" | "starttimergametime" => 2,
            "canceltimer" | "canceltimergametime" | "registerforupdate" | "registerforsingleupdate"
            | "registerforupdategametime" | "registerforsingleupdategametime" => 1,
            _ => 0,
        }
    }
}

/// Every script instance's update registrations, and the clocks they count.
/// Saved.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct ScriptUpdates {
    registrations: Vec<UpdateRegistration>,
    /// Simulation seconds counted so far.
    seconds: f64,
    /// The game clock (hours since the runtime began) last reported.
    game_hours: Option<f64>,
}

impl Resource for ScriptUpdates {}

impl ScriptUpdates {
    fn now(&self, clock: UpdateClock) -> Option<f64> {
        match clock {
            UpdateClock::RealTime => Some(self.seconds),
            UpdateClock::GameTime => self.game_hours,
        }
    }

    /// Apply `request` from `owner`'s instance of `script`.
    pub fn request(&mut self, owner: u32, script: &str, request: UpdateRequest) {
        let mine = |r: &UpdateRegistration, clock: UpdateClock| {
            r.owner == owner && r.clock == clock && r.script.eq_ignore_ascii_case(script)
        };
        match request {
            UpdateRequest::Register {
                clock,
                interval,
                recurring,
            } => {
                self.registrations
                    .retain(|r| !(mine(r, clock) && r.timer.is_none() && r.recurring == recurring));
                let interval = interval.max(0.0);
                self.registrations.push(UpdateRegistration {
                    owner,
                    script: script.to_owned(),
                    clock,
                    recurring,
                    timer: None,
                    interval,
                    due: self.now(clock).map(|now| now + f64::from(interval)),
                });
            }
            UpdateRequest::Unregister(clock) => self.registrations.retain(|r| !(mine(r, clock) && r.timer.is_none())),
            UpdateRequest::StartTimer { clock, interval, id } => {
                self.registrations.retain(|r| !(mine(r, clock) && r.timer == Some(id)));
                let interval = match clock {
                    UpdateClock::RealTime => interval.max(0.0),
                    UpdateClock::GameTime => interval.max(MIN_GAME_TIMER_HOURS),
                };
                self.registrations.push(UpdateRegistration {
                    owner,
                    script: script.to_owned(),
                    clock,
                    recurring: false,
                    timer: Some(id),
                    interval,
                    due: self.now(clock).map(|now| now + f64::from(interval)),
                });
            }
            UpdateRequest::CancelTimer { clock, id } => {
                self.registrations.retain(|r| !(mine(r, clock) && r.timer == Some(id)))
            }
        }
    }

    /// The game clock (hours since the runtime began) the host last
    /// reported; `None` before the first report.
    pub fn game_hours(&self) -> Option<f64> {
        self.game_hours
    }

    pub fn registrations(&self) -> &[UpdateRegistration] {
        &self.registrations
    }

    /// Advance the clocks and take the registrations now due, re-arming the
    /// periodic ones. `held` keeps a due registration for a later tick.
    fn take_due(&mut self, dt: f32, game_hours: Option<f64>, held: impl Fn(&UpdateRegistration) -> Hold) -> Vec<UpdateRegistration> {
        self.seconds += f64::from(dt.max(0.0));
        if let Some(hours) = game_hours.filter(|hours| hours.is_finite()) {
            self.game_hours = Some(hours);
        }
        let mut due = Vec::new();
        let (seconds, game_hours) = (self.seconds, self.game_hours);
        self.registrations.retain_mut(|r| {
            let now = match r.clock {
                UpdateClock::RealTime => seconds,
                UpdateClock::GameTime => match game_hours {
                    Some(hours) => hours,
                    None => return true,
                },
            };
            let due_at = *r.due.get_or_insert(now + f64::from(r.interval));
            if due_at > now {
                return true;
            }
            match held(r) {
                Hold::Drop => return false,
                Hold::Wait => return true,
                Hold::Deliver => {}
            }
            due.push(r.clone());
            if !r.recurring {
                return false;
            }
            // One update per tick, however far the clock jumped.
            r.due = Some(now + f64::from(r.interval.max(f32::EPSILON)));
            true
        });
        due
    }
}

/// What a due registration does this tick.
enum Hold {
    Deliver,
    /// Its reference is unloaded: deliver when it loads.
    Wait,
    /// Its quest stopped.
    Drop,
}

/// Per (script, handler, on a quest): whether the Papyrus VM runs the
/// handler, checked at its first delivery against the installed scripts.
/// Session-only.
#[derive(Debug, Default)]
pub struct UpdateHandlerChecks(HashMap<(String, &'static str, bool), bool>);

impl Resource for UpdateHandlerChecks {}

/// Deliver the update events now due. `game_hours` is the host's absolute
/// game clock in hours, when it has one.
pub fn script_update_system(world: &World, dt: f32, game_hours: Option<f64>) {
    let quests = world
        .try_resource::<crate::QuestScriptInstances>()
        .map(|instances| Arc::clone(&instances.0))
        .unwrap_or_default();
    let fragments = world.resource::<QuestStageFragments>().clone();
    // The loaded references' scripts, by FormID.
    let mut references: HashMap<u32, Vec<ReferenceScript>> = HashMap::new();
    if let Some(attached) = world.query::<PapyrusReferenceScripts>() {
        for (entity, scripts) in attached.iter() {
            if let Some(form) = crate::condition::entity_global_form_id(world, entity) {
                references.entry(form).or_default().extend(scripts.0.iter().cloned());
            }
        }
    }
    // Each owner's standing, read before the registrations are written: a
    // quest delivers while it runs, a reference while it is loaded.
    let owners: Vec<u32> = match world.try_resource::<ScriptUpdates>() {
        Some(updates) => updates.registrations.iter().map(|r| r.owner).collect(),
        None => return,
    };
    let holds: HashMap<u32, bool> = {
        let stages = world.resource::<QuestStageState>();
        owners
            .into_iter()
            .filter_map(|owner| {
                let quest = QuestFormId(owner);
                let is_quest = quests.contains_key(&quest) || fragments.vmad(quest).is_some() || stages.is_started(quest);
                is_quest.then(|| (owner, stages.is_running(quest)))
            })
            .collect()
    };
    let due = world.resource_mut::<ScriptUpdates>().take_due(dt, game_hours, |r| match holds.get(&r.owner) {
        Some(true) => Hold::Deliver,
        Some(false) => Hold::Drop,
        None if references.contains_key(&r.owner) => Hold::Deliver,
        None => Hold::Wait,
    });
    if due.is_empty() {
        return;
    }
    let mut advances = Vec::new();
    for registration in due {
        let owner = QuestFormId(registration.owner);
        let on_quest = holds.contains_key(&registration.owner);
        let event = registration.clock.event(registration.timer.is_some());
        let key = (registration.script.to_ascii_lowercase(), event, on_quest);
        let cached = world
            .try_resource::<UpdateHandlerChecks>()
            .and_then(|checks| checks.0.get(&key).copied());
        let runs = cached.unwrap_or_else(|| {
            let runs = super::vm::handler_runs(&fragments.vm_scripts(), &registration.script, event, on_quest);
            if let Some(mut checks) = world.try_resource_mut::<UpdateHandlerChecks>() {
                checks.0.insert(key, runs);
            }
            runs
        });
        if !runs {
            log::debug!(
                "{:08X}: {}.{event} skipped: the Papyrus VM does not run it",
                registration.owner,
                registration.script
            );
            continue;
        }
        let vmad = match quests.get(&owner).or_else(|| fragments.vmad(owner)) {
            Some(vmad) => Some(vmad.clone()),
            None => references.get(&registration.owner).and_then(|scripts| {
                scripts
                    .iter()
                    .find(|script| script.script.eq_ignore_ascii_case(&registration.script))
                    .map(|script| (*script.vmad).clone())
            }),
        };
        let effect = Effect::Interpret {
            script: registration.script.clone(),
            function: event.to_owned(),
            args: registration
                .timer
                .map(|id| vec![crate::translate::effects::VmStored::Int(id)])
                .unwrap_or_default(),
            resume: None,
        };
        log::debug!("{:08X}: {}.{event}", registration.owner, registration.script);
        advances.extend(apply_fragment_guard_free(
            world,
            std::slice::from_ref(&effect),
            owner,
            vmad.as_ref(),
        ));
    }
    if advances.is_empty() {
        return;
    }
    let Some(player) = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0)
    else {
        return;
    };
    crate::quest_stages::push_quest_stage_advances(world, player, advances);
}
