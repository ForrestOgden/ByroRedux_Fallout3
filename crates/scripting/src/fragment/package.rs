//! M47.2 — package (`PACK`) fragments: the `PF_` script functions a package
//! binds to its Begin / End / Change events, run on the Papyrus VM with
//! `akActor` bound to the actor running the package.
//!
//! The CK's *Package Fragments*: "run when the package begins, when the
//! package ends, and when the package changes"; the FO4 CK's *OnEnd* /
//! *OnChange - Package*: End is a normal end ("but not if it is
//! preempted"), Change is either. The host reports each actor's package
//! transitions ([`PendingPackageTransitions`]); a switch from one package to
//! another is a preemption — this engine re-selects packages by their
//! conditions and schedules and does not model a procedure completing — so
//! it runs the old package's Change fragment and the new one's Begin. End
//! fragments are installed but not run until procedure completion lands.
//!
//! Fragments install only when the VM's check passes (the lowering's INFO
//! speaker binding does not cover `akActor`); the fragment runs with the
//! package as its `Self`.

use super::*;
use crate::translate::effects::VmStored;
use byroredux_plugin::esm::records::script_instance::PackageFragmentEvent;

/// One package's installed fragments.
#[derive(Debug, Clone)]
pub struct PackageFragmentSet {
    /// The `PF_` script's properties.
    pub vmad: Option<ScriptInstanceData>,
    /// (event, script, function) per installed fragment.
    pub fragments: Vec<(PackageFragmentEvent, String, String)>,
}

/// Every package's installed fragments, by package FormID.
#[derive(Debug, Clone, Default)]
pub struct PackageFragments {
    map: Arc<HashMap<u32, PackageFragmentSet>>,
    populated: bool,
}

impl Resource for PackageFragments {}

impl PackageFragments {
    pub fn insert(&mut self, package: u32, set: PackageFragmentSet) {
        Arc::make_mut(&mut self.map).insert(package, set);
    }

    pub fn get(&self, package: u32) -> Option<&PackageFragmentSet> {
        self.map.get(&package)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn is_populated(&self) -> bool {
        self.populated
    }

    pub fn mark_populated(&mut self) {
        self.populated = true;
    }
}

/// Install `package`'s fragments from its decompiled `PF_` `script`: the
/// `bindings` whose function the VM's check accepts, with the scripts their
/// runs need handed to `install`. Returns how many installed.
pub fn populate_package_fragments(
    frags: &mut PackageFragments,
    package: u32,
    vmad: Option<&ScriptInstanceData>,
    script: Arc<Script>,
    bindings: &[(PackageFragmentEvent, &str)],
    library: &dyn crate::translate::effects::ScriptLibrary,
    install: &mut dyn FnMut(Arc<Script>),
) -> usize {
    let mut fragments = Vec::new();
    for (event, function) in bindings {
        let Some(scripts) = super::vm::check_function(Arc::clone(&script), function, Some(library)) else {
            continue;
        };
        for needed in scripts {
            install(needed);
        }
        fragments.push((*event, script.name.node.0.clone(), (*function).to_owned()));
    }
    let installed = fragments.len();
    if installed > 0 {
        frags.insert(
            package,
            PackageFragmentSet {
                vmad: vmad.cloned(),
                fragments,
            },
        );
    }
    installed
}

/// An actor's package changed: `from` stopped running, `to` started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageTransition {
    pub actor: EntityId,
    pub from: Option<u32>,
    pub to: Option<u32>,
}

/// The package transitions the host reported this frame, drained by
/// [`package_fragment_system`].
#[derive(Debug, Default)]
pub struct PendingPackageTransitions(pub Vec<PackageTransition>);

impl Resource for PendingPackageTransitions {}

/// Report that `actor`'s package changed from `from` to `to`.
pub fn report_package_transition(world: &World, actor: EntityId, from: Option<u32>, to: Option<u32>) {
    if from == to {
        return;
    }
    if let Some(mut pending) = world.try_resource_mut::<PendingPackageTransitions>() {
        pending.0.push(PackageTransition { actor, from, to });
    }
}

/// Run the package fragments of this frame's transitions.
pub fn package_fragment_system(world: &World, _dt: f32) {
    let transitions = match world.try_resource_mut::<PendingPackageTransitions>() {
        Some(mut pending) if !pending.0.is_empty() => std::mem::take(&mut pending.0),
        _ => return,
    };
    let fragments = world.resource::<PackageFragments>().clone();
    log::debug!("{} package transition(s), {} package(s) with fragments", transitions.len(), fragments.len());
    if fragments.is_empty() {
        return;
    }
    let player = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0);
    let mut advances = Vec::new();
    for transition in transitions {
        let actor = if Some(transition.actor) == player {
            Some(0x14)
        } else {
            crate::condition::entity_global_form_id(world, transition.actor)
        };
        let Some(actor) = actor else {
            continue;
        };
        let runs = [
            (transition.from, PackageFragmentEvent::Change),
            (transition.to, PackageFragmentEvent::Begin),
        ];
        for (package, event) in runs {
            let Some(package) = package else {
                continue;
            };
            let Some(set) = fragments.get(package) else {
                continue;
            };
            for (_, script, function) in set.fragments.iter().filter(|(on, _, _)| *on == event) {
                log::debug!("package {package:08X} {event:?} on {actor:08X}: {script}.{function}");
                let effect = Effect::Interpret {
                    script: script.clone(),
                    function: function.clone(),
                    args: vec![VmStored::Form(actor)],
                    resume: None,
                };
                advances.extend(apply_fragment_guard_free(
                    world,
                    std::slice::from_ref(&effect),
                    QuestFormId(package),
                    set.vmad.as_ref(),
                ));
            }
        }
    }
    let Some(player) = player else {
        return;
    };
    if !advances.is_empty() {
        crate::quest_stages::push_quest_stage_advances(world, player, advances);
    }
}
