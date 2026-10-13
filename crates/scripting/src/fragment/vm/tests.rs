use super::*;
use crate::papyrus_demo::PapyrusPlayerEntity;
use crate::translate::effects::Effect;
use byroredux_core::ecs::components::FormIdComponent;
use byroredux_core::form_id::{FormIdPair, FormIdPool, LocalFormId, PluginId};

const Q: QuestFormId = QuestFormId(0x0001_0F00);
const GUARD: u32 = 0x0001_0F01;
const MARKER: u32 = 0x0001_0F02;

const SOURCE: &str = "ScriptName QF_VmTest extends Quest\n\
    ReferenceAlias Property Alias_Guard Auto\n\
    ObjectReference Property Marker Auto\n\
    Function Fragment_1()\n\
      ObjectReference guard = Alias_Guard.GetReference()\n\
      If guard\n\
        guard.Disable()\n\
      EndIf\n\
      Int count = 0\n\
      While count < 3\n\
        count += 1\n\
      EndWhile\n\
      If count == Self.GetStage() - 7\n\
        Self.SetStage(20)\n\
      EndIf\n\
      Marker.Enable()\n\
    EndFunction\n";

fn parsed() -> Arc<Script> {
    let (script, errors) = byroredux_papyrus::parse_script(SOURCE).expect("parses");
    assert!(errors.is_empty(), "{errors:?}");
    Arc::new(script)
}

fn vmad() -> ScriptInstanceData {
    ScriptInstanceData {
        version: 5,
        object_format: 2,
        scripts: vec![ScriptInstance {
            name: "QF_VmTest".into(),
            status: 0,
            properties: vec![
                ScriptProperty {
                    name: "Alias_Guard".into(),
                    status: 1,
                    value: PropertyValue::Object {
                        form_id: Q.0,
                        alias: 0,
                    },
                },
                ScriptProperty {
                    name: "Marker".into(),
                    status: 1,
                    value: PropertyValue::Object {
                        form_id: MARKER,
                        alias: -1,
                    },
                },
            ],
        }],
    }
}

/// The lowering declines this fragment (an object local from a call, a
/// counting loop, a computed comparison); the VM's check takes it.
#[test]
fn the_check_takes_what_the_lowering_declines() {
    let script = parsed();
    let body = callable_in(&script, None, "Fragment_1").unwrap().body.to_vec();
    assert_eq!(
        crate::translate::effects::lower_fragment(&body),
        None,
        "the lowering declines it"
    );
    let scripts = check_function(Arc::clone(&script), "Fragment_1", None).expect("the VM runs it");
    assert_eq!(scripts.len(), 1);
    // A function called as a statement may wait; one called for its value
    // may not.
    let (waits, _) = byroredux_papyrus::parse_script(
        "ScriptName QF_Wait extends Quest\nFunction Fragment_1()\n Utility.Wait(1.0)\n Helper()\n EndFunction\n\
         Function Fragment_2()\n Int n = Helper()\n EndFunction\n\
         Int Function Helper()\n Utility.Wait(1.0)\n Return 1\n EndFunction\n",
    )
    .unwrap();
    let waits = Arc::new(waits);
    assert!(check_function(Arc::clone(&waits), "Fragment_1", None).is_some());
    assert_eq!(
        check_function_reason(waits, "Fragment_2", None).err().as_deref(),
        Some("a wait in a function called for its value")
    );
}

/// Installed as `Effect::Interpret`, the fragment runs through the stage
/// dispatcher: the alias's reference is disabled, the loop counts to 3, the
/// computed comparison against `GetStage()` (10 - 7) holds and sets stage 20,
/// and the marker is enabled.
#[test]
fn an_interpreted_fragment_runs_through_stage_dispatch() {
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(GUARD),
    });
    world.insert_resource(pool);
    let guard = world.spawn();
    world.insert(guard, FormIdComponent(id));
    world
        .resource_mut::<crate::SceneActorBindings>()
        .bind(Q, 0, guard);
    world
        .resource_mut::<crate::fragment::ReferenceEnableState>()
        .set_enabled(MARKER, false);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        fragments.insert_vmad(Q, vmad());
        fragments.insert_vm_script(parsed());
        fragments.insert(
            Q,
            10,
            vec![Effect::Interpret {
                script: "QF_VmTest".into(),
                function: "Fragment_1".into(),
                args: Vec::new(),
                resume: None,
            }],
        );
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, GUARD), "the alias's reference");
    assert!(crate::fragment::reference_enabled(&world, MARKER));
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
}

/// An object script variable holds its reference from one fragment to the
/// next (`QuestScriptObjects`), as Papyrus script state does.
#[test]
fn object_script_variables_outlive_the_fragment_that_wrote_them() {
    const SOURCE: &str = "ScriptName QF_VmObjects extends Quest\n\
        ReferenceAlias Property Alias_Guard Auto\n\
        ObjectReference Stored\n\
        Function Fragment_1()\n Stored = Alias_Guard.GetReference()\n EndFunction\n\
        Function Fragment_2()\n Stored.Disable()\n EndFunction\n";
    let (script, errors) = byroredux_papyrus::parse_script(SOURCE).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let script = Arc::new(script);
    for fragment in ["Fragment_1", "Fragment_2"] {
        assert!(check_function(Arc::clone(&script), fragment, None).is_some(), "{fragment}");
    }
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.insert_resource(crate::QuestScriptObjects::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(GUARD),
    });
    world.insert_resource(pool);
    let guard = world.spawn();
    world.insert(guard, FormIdComponent(id));
    world.resource_mut::<crate::SceneActorBindings>().bind(Q, 0, guard);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        fragments.insert_vmad(Q, vmad());
        fragments.insert_vm_script(Arc::clone(&script));
        for (stage, function) in [(10, "Fragment_1"), (20, "Fragment_2")] {
            fragments.insert(
                Q,
                stage,
                vec![Effect::Interpret {
                    script: "QF_VmObjects".into(),
                    function: function.into(),
                    args: Vec::new(),
                resume: None,
                }],
            );
        }
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    // The alias is cleared before the second fragment: the variable keeps it.
    *world.resource_mut::<crate::SceneActorBindings>() = Default::default();
    world.resource_mut::<QuestStageState>().set_stage(Q, 20);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, GUARD));
}

/// `Utility.Wait` suspends the run; each resume continues at the wait — here
/// one at the top level, then one inside an `If` branch — with the locals it
/// had.
#[test]
fn waits_suspend_and_resume_where_they_were() {
    const SOURCE: &str = "ScriptName QF_VmWaits extends Quest\n\
        ObjectReference Property Marker Auto\n\
        Function Fragment_1()\n\
          Int n = 1\n\
          Utility.Wait(1.0)\n\
          If n == 1\n\
            Marker.Enable()\n\
            Utility.Wait(2.0)\n\
            Self.SetStage(20)\n\
          EndIf\n\
        EndFunction\n";
    let (script, errors) = byroredux_papyrus::parse_script(SOURCE).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let script = Arc::new(script);
    assert!(check_function(Arc::clone(&script), "Fragment_1", None).is_some());
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world
        .resource_mut::<crate::fragment::ReferenceEnableState>()
        .set_enabled(MARKER, false);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        fragments.insert_vmad(Q, vmad());
        fragments.insert_vm_script(Arc::clone(&script));
        fragments.insert(
            Q,
            10,
            vec![Effect::Interpret {
                script: "QF_VmWaits".into(),
                function: "Fragment_1".into(),
                args: Vec::new(),
                resume: None,
            }],
        );
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, MARKER), "waiting 1 s");
    crate::fragment::fragment_continuation_system(&world, 1.0);
    assert!(crate::fragment::reference_enabled(&world, MARKER), "resumed into the If");
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 10, "waiting 2 s");
    crate::fragment::fragment_continuation_system(&world, 2.0);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
}

/// A wait inside a called function suspends the whole call chain; the resume
/// finishes the callee — its locals intact — and then the caller after the
/// call.
#[test]
fn a_wait_in_a_called_function_resumes_the_call_chain() {
    const SOURCE: &str = "ScriptName QF_VmCallWaits extends Quest\n\
        ObjectReference Property Marker Auto\n\
        Function Fragment_1()\n\
          Helper(2)\n\
          Self.SetStage(30)\n\
        EndFunction\n\
        Function Helper(Int target)\n\
          Utility.Wait(1.0)\n\
          Marker.Enable()\n\
          If target == 2\n\
            Utility.Wait(1.0)\n\
            Self.SetStage(20)\n\
          EndIf\n\
        EndFunction\n";
    let (script, errors) = byroredux_papyrus::parse_script(SOURCE).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let script = Arc::new(script);
    assert!(check_function(Arc::clone(&script), "Fragment_1", None).is_some());
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world
        .resource_mut::<crate::fragment::ReferenceEnableState>()
        .set_enabled(MARKER, false);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        fragments.insert_vmad(Q, vmad());
        fragments.insert_vm_script(Arc::clone(&script));
        fragments.insert(
            Q,
            10,
            vec![Effect::Interpret {
                script: "QF_VmCallWaits".into(),
                function: "Fragment_1".into(),
                args: Vec::new(),
                resume: None,
            }],
        );
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, MARKER), "waiting in Helper");
    crate::fragment::fragment_continuation_system(&world, 1.0);
    assert!(crate::fragment::reference_enabled(&world, MARKER), "Helper resumed");
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 10, "waiting in Helper's If");
    crate::fragment::fragment_continuation_system(&world, 1.0);
    // Helper sets 20 with the parameter it was called with, then the caller
    // continues after the call and sets 30.
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 30);
}

/// `Global.Value = n` is `Global.SetValue(n)`; with `+=` it counts from the
/// global's current value.
#[test]
fn global_value_assignment_writes_the_global() {
    const COUNT: u32 = 0x0001_0F03;
    const SOURCE: &str = "ScriptName QF_VmGlobals extends Quest\n\
        GlobalVariable Property Count Auto\n\
        Function Fragment_1()\n\
          Count.Value += 2\n\
          Count.Value = Count.Value * 10\n\
        EndFunction\n";
    let (script, errors) = byroredux_papyrus::parse_script(SOURCE).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let script = Arc::new(script);
    assert_eq!(
        check_function_reason(Arc::clone(&script), "Fragment_1", None).err(),
        None
    );
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    let mut globals = crate::Globals::default();
    globals.set(COUNT, 1.0);
    world.insert_resource(globals);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        fragments.insert_vmad(
            Q,
            ScriptInstanceData {
                version: 5,
                object_format: 2,
                scripts: vec![ScriptInstance {
                    name: "QF_VmGlobals".into(),
                    status: 0,
                    properties: vec![ScriptProperty {
                        name: "Count".into(),
                        status: 1,
                        value: PropertyValue::Object {
                            form_id: COUNT,
                            alias: -1,
                        },
                    }],
                }],
            },
        );
        fragments.insert_vm_script(Arc::clone(&script));
        fragments.insert(
            Q,
            10,
            vec![Effect::Interpret {
                script: "QF_VmGlobals".into(),
                function: "Fragment_1".into(),
                args: Vec::new(),
                resume: None,
            }],
        );
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<crate::Globals>().get(COUNT), Some(30.0));
}

/// A one-quest world with `script` installed for `Q`'s stage-10 fragment
/// `Fragment_1`, `MARKER` disabled.
fn world_with_fragment(script: Arc<Script>) -> World {
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.insert_resource(crate::QuestScriptObjects::default());
    world
        .resource_mut::<crate::fragment::ReferenceEnableState>()
        .set_enabled(MARKER, false);
    let name = script.name.node.0.clone();
    let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
    fragments.insert_vmad(Q, vmad());
    fragments.insert_vm_script(script);
    fragments.insert(
        Q,
        10,
        vec![Effect::Interpret {
            script: name,
            function: "Fragment_1".into(),
            args: Vec::new(),
            resume: None,
        }],
    );
    drop(fragments);
    world
}

fn parsed_source(source: &str) -> Arc<Script> {
    let (script, errors) = byroredux_papyrus::parse_script(source).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    Arc::new(script)
}

/// A call runs the version of the current state (the auto state at first);
/// `GotoState` switches it and runs the new state's `OnBeginState`.
#[test]
fn calls_run_the_version_of_the_current_state() {
    let script = parsed_source(
        "ScriptName QF_VmStates extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Function Fragment_1()\n Ping()\n Ping()\n EndFunction\n\
         Function Ping()\n EndFunction\n\
         Auto State Waiting\n\
           Function Ping()\n Self.SetStage(20)\n GotoState(\"Done\")\n EndFunction\n\
         EndState\n\
         State Done\n\
           Event OnBeginState()\n Marker.Enable()\n EndEvent\n\
           Function Ping()\n If GetState() == \"Done\"\n Self.SetStage(30)\n EndIf\n EndFunction\n\
         EndState\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let world = world_with_fragment(script);
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(crate::fragment::reference_enabled(&world, MARKER), "Done's OnBeginState");
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 30, "the second Ping ran Done's version");
}

/// An unwritten script variable or property reads what the script declares;
/// a `String` variable holds what a script writes to it.
#[test]
fn script_variables_read_their_declared_values_and_hold_strings() {
    let script = parsed_source(
        "ScriptName QF_VmDeclared extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Int Property Goal = 3 Auto\n\
         String Label = \"start\"\n\
         Function Fragment_1()\n\
           If Goal == 3 && Label == \"start\"\n Label = \"next\"\n EndIf\n\
           If Label == \"next\"\n Marker.Enable()\n EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let world = world_with_fragment(script);
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(crate::fragment::reference_enabled(&world, MARKER));
}

/// A reference script the VM runs: `OnInit` once, `OnLoad` on every cell
/// load, `OnTriggerEnter` only in its auto state (the crossing moves it to
/// `Done`), and `OnActivate` — each through the event markers.
#[test]
fn reference_scripts_run_their_events() {
    const REF: u32 = 0x0001_0F10;
    let script = parsed_source(
        "ScriptName VmTriggerScript extends ObjectReference\n\
         ObjectReference Property Marker Auto\n\
         Int Property Count Auto\n\
         Event OnInit()\n Count = 10\n EndEvent\n\
         Event OnLoad()\n Count += 1\n EndEvent\n\
         Event OnActivate(ObjectReference akActionRef)\n Self.Disable()\n EndEvent\n\
         Auto State Waiting\n\
           Event OnTriggerEnter(ObjectReference akActionRef)\n\
             If akActionRef == Game.GetPlayer()\n GotoState(\"Done\")\n Marker.Enable()\n EndIf\n\
           EndEvent\n\
         EndState\n\
         State Done\n EndState\n",
    );
    let (events, scripts) =
        crate::check_reference_script(Arc::clone(&script), &crate::DELIVERED_EVENTS, None).expect("the VM runs it");
    assert_eq!(events, ["OnInit", "OnLoad", "OnActivate", "OnTriggerEnter"]);

    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(REF),
    });
    world.insert_resource(pool);
    let reference = world.spawn();
    world.insert(reference, FormIdComponent(id));
    world
        .resource_mut::<crate::fragment::ReferenceEnableState>()
        .set_enabled(MARKER, false);
    let instance = ScriptInstance {
        name: "VmTriggerScript".into(),
        status: 0,
        properties: vec![ScriptProperty {
            name: "Marker".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: MARKER,
                alias: -1,
            },
        }],
    };
    crate::attach_reference_script(&mut world, reference, &instance, events, scripts);

    let count = |world: &World| {
        world
            .resource::<crate::QuestScriptVariables>()
            .get(QuestFormId(REF), crate::QuestScriptVariables::property_key("Count"))
    };
    for expected in [11.0, 12.0] {
        world.insert(reference, crate::OnCellLoadEvent);
        crate::reference_script_event_system(&world, 0.0);
        world.remove::<crate::OnCellLoadEvent>(reference);
        assert_eq!(count(&world), Some(expected), "OnInit once, OnLoad each load");
    }

    for crossing in 0..2 {
        world.insert(
            reference,
            crate::OnTriggerEnterEvent {
                triggerers: vec![player],
            },
        );
        crate::reference_script_event_system(&world, 0.0);
        world.remove::<crate::OnTriggerEnterEvent>(reference);
        assert_eq!(
            crate::fragment::reference_enabled(&world, MARKER),
            crossing == 0,
            "only the auto state handles the crossing"
        );
        world
            .resource_mut::<crate::fragment::ReferenceEnableState>()
            .set_enabled(MARKER, false);
    }

    world.insert(reference, crate::ActivateEvent { activator: player });
    crate::reference_script_event_system(&world, 0.0);
    assert!(!crate::fragment::reference_enabled(&world, REF), "OnActivate disabled itself");
}

/// `RegisterForUpdate` delivers `OnUpdate` every interval until the handler
/// unregisters; the check accepts the registration because the handler runs.
#[test]
fn periodic_updates_run_until_unregistered() {
    let script = parsed_source(
        "ScriptName QF_VmUpdates extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Int Ticks\n\
         Function Fragment_1()\n RegisterForUpdate(1.0)\n EndFunction\n\
         Event OnUpdate()\n\
           Ticks += 1\n\
           If Ticks == 3\n UnregisterForUpdate()\n Marker.Enable()\n EndIf\n\
         EndEvent\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::QuestScriptVariables::default());
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    let ticks = |world: &World| {
        world
            .resource::<crate::QuestScriptVariables>()
            .get(Q, crate::QuestScriptVariables::property_key("Ticks"))
    };
    crate::script_update_system(&world, 0.5, None);
    assert_eq!(ticks(&world), None, "not due yet");
    for expected in [1.0, 2.0, 3.0, 3.0] {
        crate::script_update_system(&world, 1.0, None);
        assert_eq!(ticks(&world), Some(expected));
    }
    assert!(crate::fragment::reference_enabled(&world, MARKER));
    assert!(world.resource::<crate::ScriptUpdates>().registrations().is_empty());
}

/// A registration whose handler the VM cannot run does not fail the code
/// that registers; the delivery skips the handler whole — it never runs the
/// statements before the one it cannot.
#[test]
fn a_handler_the_vm_cannot_run_is_skipped_whole() {
    let script = parsed_source(
        "ScriptName QF_VmBadUpdate extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Function Fragment_1()\n RegisterForSingleUpdate(1.0)\n EndFunction\n\
         Event OnUpdate()\n Marker.Enable()\n Debug.SendAnimationEvent(None, \"x\")\n EndEvent\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let world = world_with_fragment(script);
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    crate::script_update_system(&world, 2.0, None);
    assert!(!crate::fragment::reference_enabled(&world, MARKER), "no partial run");
}

/// A reference's `RegisterForSingleUpdateGameTime` counts game hours from
/// the first clock the host reports, and delivers once.
#[test]
fn game_time_updates_count_game_hours() {
    const REF: u32 = 0x0001_0F11;
    let script = parsed_source(
        "ScriptName VmGameTimeScript extends ObjectReference\n\
         Event OnLoad()\n RegisterForSingleUpdateGameTime(2.0)\n EndEvent\n\
         Event OnUpdateGameTime()\n Self.Disable()\n EndEvent\n",
    );
    let (events, scripts) =
        crate::check_reference_script(Arc::clone(&script), &crate::DELIVERED_EVENTS, None).expect("runs");
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(REF),
    });
    world.insert_resource(pool);
    let reference = world.spawn();
    world.insert(reference, FormIdComponent(id));
    let instance = ScriptInstance {
        name: "VmGameTimeScript".into(),
        status: 0,
        properties: Vec::new(),
    };
    crate::attach_reference_script(&mut world, reference, &instance, events, scripts);
    world.insert(reference, crate::OnCellLoadEvent);
    crate::reference_script_event_system(&world, 0.0);
    // The registration predates the first reported clock: it arms at 10 h.
    crate::script_update_system(&world, 0.1, Some(10.0));
    crate::script_update_system(&world, 0.1, Some(11.5));
    assert!(crate::fragment::reference_enabled(&world, REF), "1.5 game hours in");
    crate::script_update_system(&world, 0.1, Some(12.0));
    assert!(!crate::fragment::reference_enabled(&world, REF));
    assert!(world.resource::<crate::ScriptUpdates>().registrations().is_empty(), "a single update");
}

/// A `Native` declaration in a parent script the library serves (`Quest`,
/// `ObjectReference`, …) is not a script body: `Self.SetStage` still runs the
/// native, never the declaration's empty body.
#[test]
fn native_declarations_in_parent_scripts_stay_natives() {
    let quest = parsed_source(
        "ScriptName Quest extends Form\n\
         Bool Function SetStage(Int aiStage) Native\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmNatives extends Quest\n\
         Function Fragment_1()\n Self.SetStage(20)\n EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("quest".to_owned(), Arc::clone(&quest))].into();
    let library = Installed(&scripts);
    let needed = check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).expect("runs");
    let mut world = world_with_fragment(script);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for needed in needed {
            fragments.insert_vm_script(needed);
        }
    }
    world.insert_resource(crate::QuestScriptVariables::default());
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
}

/// An engine class's `AutoReadOnly` constant reads its value — on any
/// reference, and unqualified in a script that inherits it.
#[test]
fn engine_constants_read_their_values() {
    let object_reference = parsed_source(
        "ScriptName ObjectReference extends Form\n\
         Int Property Motion_Keyframed = 2 AutoReadOnly\n\
         Function Disable(Bool abFadeOut = False) Native\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmConstants extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Function Fragment_1()\n\
           If Marker.Motion_Keyframed == 2\n Self.SetStage(20)\n EndIf\n\
         EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("objectreference".to_owned(), object_reference)].into();
    let library = Installed(&scripts);
    let needed = check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).expect("runs");
    let mut world = world_with_fragment(script);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for needed in needed {
            fragments.insert_vm_script(needed);
        }
    }
    world.insert_resource(crate::QuestScriptVariables::default());
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
}

/// A reference script's `OnDeath(akKiller)` runs from the victim's
/// `OnDeathEvent`, which its dispatcher drains (Pattern B).
#[test]
fn on_death_runs_with_the_killer() {
    const REF: u32 = 0x0001_0F12;
    let script = parsed_source(
        "ScriptName VmDeathScript extends Actor\n\
         ObjectReference Property Marker Auto\n\
         Event OnDeath(Actor akKiller)\n\
           If akKiller == Game.GetPlayer()\n Marker.Enable()\n EndIf\n\
         EndEvent\n",
    );
    let (events, scripts) =
        crate::check_reference_script(Arc::clone(&script), &crate::DELIVERED_EVENTS, None).expect("runs");
    assert_eq!(events, ["OnDeath"]);
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(REF),
    });
    world.insert_resource(pool);
    let victim = world.spawn();
    world.insert(victim, FormIdComponent(id));
    world
        .resource_mut::<crate::fragment::ReferenceEnableState>()
        .set_enabled(MARKER, false);
    let instance = ScriptInstance {
        name: "VmDeathScript".into(),
        status: 0,
        properties: vec![ScriptProperty {
            name: "Marker".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: MARKER,
                alias: -1,
            },
        }],
    };
    crate::attach_reference_script(&mut world, victim, &instance, events, scripts);
    world.insert(victim, crate::OnDeathEvent { killer: Some(player) });
    crate::reference_script_event_system(&world, 0.0);
    assert!(crate::fragment::reference_enabled(&world, MARKER));
    assert!(world.get::<crate::OnDeathEvent>(victim).is_none(), "drained by its consumer");
}

/// An interpreted topic-info fragment's `akSpeakerRef` is the line's speaker.
#[test]
fn info_fragments_bind_the_speaker() {
    let script = parsed_source(
        "ScriptName TIF__VmSpeaker extends TopicInfo\n\
         Function Fragment_0(ObjectReference akSpeakerRef)\n\
           Actor akSpeaker = akSpeakerRef as Actor\n\
           akSpeaker.Disable()\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_0", None).err(), None);
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(GUARD),
    });
    world.insert_resource(pool);
    let speaker = world.spawn();
    world.insert(speaker, FormIdComponent(id));
    world.resource_mut::<crate::QuestStageFragments>().insert_vm_script(script);
    let effect = Effect::Interpret {
        script: "TIF__VmSpeaker".into(),
        function: "Fragment_0".into(),
        args: Vec::new(),
        resume: None,
    };
    crate::apply_spoken_info_fragment(&world, std::slice::from_ref(&effect), Q, None, Some(speaker));
    assert!(!crate::fragment::reference_enabled(&world, GUARD));
}

/// A fragment calls a function of the script the VM runs on another
/// reference, reads its variable and writes another: the call runs as that
/// reference's (its `Self`, its variables), the writes land under it.
#[test]
fn calls_and_members_reach_scripts_on_other_references() {
    const TRAP: u32 = 0x0001_0F13;
    let trap_script = parsed_source(
        "ScriptName VmTrapBase extends ObjectReference\n\
         Int Property Hits Auto\n\
         Bool Armed = True\n\
         Function Fire()\n\
           If Armed\n Hits += 1\n Self.Disable()\n EndIf\n\
         EndFunction\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmCrossRef extends Quest\n\
         ObjectReference Property Trap Auto\n\
         Function Fragment_1()\n\
           VmTrapBase t = Trap as VmTrapBase\n\
           t.Fire()\n\
           If t.Hits == 1\n (Trap as VmTrapBase).Armed = False\n Self.SetStage(20)\n EndIf\n\
         EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("vmtrapbase".to_owned(), Arc::clone(&trap_script))].into();
    let library = Installed(&scripts);
    let needed = check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).expect("runs");
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::QuestScriptVariables::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(TRAP),
    });
    world.insert_resource(pool);
    let trap = world.spawn();
    world.insert(trap, FormIdComponent(id));
    let instance = ScriptInstance {
        name: "VmTrapBase".into(),
        status: 0,
        properties: Vec::new(),
    };
    crate::attach_reference_script(&mut world, trap, &instance, Vec::new(), vec![trap_script]);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for needed in needed {
            fragments.insert_vm_script(needed);
        }
        // The fragment's `Trap` property.
        let mut vmad = vmad();
        vmad.scripts[0].properties.push(ScriptProperty {
            name: "Trap".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: TRAP,
                alias: -1,
            },
        });
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, TRAP), "Fire ran as the trap");
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20, "read the trap's Hits");
    let variable = |name: &str| {
        world
            .resource::<crate::QuestScriptVariables>()
            .get(QuestFormId(TRAP), crate::QuestScriptVariables::property_key(name))
    };
    assert_eq!(variable("Hits"), Some(1.0));
    assert_eq!(variable("Armed"), Some(0.0), "written under the trap");
}

/// `Game.IncrementStat` counts a misc stat; `Game.QueryStat` reads a
/// script-kept one, increments queued earlier in the run included.
#[test]
fn misc_stats_count_and_read() {
    let script = parsed_source(
        "ScriptName QF_VmStats extends Quest\n\
         Function Fragment_1()\n\
           Game.IncrementStat(\"Houses Owned\")\n\
           Game.IncrementStat(\"Houses Owned\", 2)\n\
           If Game.QueryStat(\"Houses Owned\") == 3\n Self.SetStage(20)\n EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let world = world_with_fragment(script);
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
    assert_eq!(world.resource::<crate::MiscStats>().get("houses owned"), 3);
    // An engine-tracked stat is not counted here: the read declines.
    let days = parsed_source(
        "ScriptName QF_VmDays extends Quest\n\
         Function Fragment_1()\n If Game.QueryStat(\"Days Passed\") > 1\n Self.SetStage(20)\n EndIf\n EndFunction\n",
    );
    assert!(check_function(days, "Fragment_1", None).is_none());
}

/// `OnHit` receives the aggressor, the aggressor's equipped weapon as the
/// source, and the hit's flags.
#[test]
fn on_hit_receives_the_aggressor_weapon_and_flags() {
    const REF: u32 = 0x0001_0F14;
    const SWORD: u32 = 0x0001_2EB7;
    let script = parsed_source(
        "ScriptName VmHitScript extends ObjectReference\n\
         Weapon Property Sword Auto\n\
         Event OnHit(ObjectReference akAggressor, Form akSource, Projectile akProjectile, \
           Bool abPowerAttack, Bool abSneakAttack, Bool abBashAttack, Bool abHitBlocked)\n\
           If akAggressor == Game.GetPlayer() && akSource == Sword && abPowerAttack && !abHitBlocked\n\
             Self.Disable()\n\
           EndIf\n\
         EndEvent\n",
    );
    let (events, scripts) =
        crate::check_reference_script(Arc::clone(&script), &crate::DELIVERED_EVENTS, None).expect("runs");
    assert_eq!(events, ["OnHit"]);
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    world.register::<byroredux_core::ecs::components::EquippedWeapon>();
    world.insert(
        player,
        byroredux_core::ecs::components::EquippedWeapon {
            inventory_index: byroredux_core::ecs::components::InventoryIndex(0),
            base_form_id: SWORD,
            damage: 7.0,
            reach: 0.0,
            speed: 0.0,
        },
    );
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(REF),
    });
    world.insert_resource(pool);
    let target = world.spawn();
    world.insert(target, FormIdComponent(id));
    let instance = ScriptInstance {
        name: "VmHitScript".into(),
        status: 0,
        properties: vec![ScriptProperty {
            name: "Sword".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: SWORD,
                alias: -1,
            },
        }],
    };
    crate::attach_reference_script(&mut world, target, &instance, events, scripts);
    world.insert(
        target,
        crate::HitEvent {
            aggressor: player,
            source: player,
            projectile: player,
            damage: 7.0,
            power_attack: true,
            sneak_attack: false,
            bash_attack: false,
            blocked: false,
        },
    );
    crate::reference_script_event_system(&world, 0.0);
    assert!(!crate::fragment::reference_enabled(&world, REF));
}

/// An engine constant argument passes to a native by name, so a per-game
/// number (`Motion_Keyframed` is 4 in Skyrim) never reaches the primitives
/// as a bare literal they cannot place.
#[test]
fn engine_constant_arguments_pass_by_name() {
    let object_reference = parsed_source(
        "ScriptName ObjectReference extends Form\n\
         Int Property Motion_Keyframed = 4 AutoReadOnly\n\
         Function SetMotionType(Int aiMotionType, Bool abAllowActivate = True) Native\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmMotion extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Function Fragment_1()\n Marker.SetMotionType(Marker.Motion_Keyframed, True)\n EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("objectreference".to_owned(), object_reference)].into();
    let library = Installed(&scripts);
    let needed = check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).expect("runs");
    let mut world = world_with_fragment(script);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for needed in needed {
            fragments.insert_vm_script(needed);
        }
    }
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(MARKER),
    });
    world.insert_resource(pool);
    let marker = world.spawn();
    world.insert(marker, FormIdComponent(id));
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    let request = world
        .get::<crate::MotionTypeChangeRequest>(marker)
        .map(|request| request.motion_type);
    assert_eq!(request, Some(byroredux_core::ecs::components::MotionType::Keyframed));
}

/// Fallout 4 timers: `StartTimer(afInterval, aiTimerID)` fires `OnTimer` once
/// with its ID; `CancelTimer` stops one; a game-time interval under 0.033
/// hours rounds up to it.
#[test]
fn timers_fire_once_with_their_id() {
    let script = parsed_source(
        "ScriptName QF_VmTimers extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Int Fired\n\
         Function Fragment_1()\n\
           StartTimer(5.0, 7)\n\
           StartTimer(1.0, 8)\n\
           CancelTimer(8)\n\
           StartTimerGameTime(0.01, 3)\n\
         EndFunction\n\
         Event OnTimer(Int aiTimerID)\n\
           If aiTimerID == 7\n Marker.Enable()\n EndIf\n\
           Fired += 1\n\
         EndEvent\n\
         Event OnTimerGameTime(Int aiTimerID)\n\
           If aiTimerID == 3\n Self.SetStage(20)\n EndIf\n\
         EndEvent\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::QuestScriptVariables::default());
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    let fired = |world: &World| {
        world
            .resource::<crate::QuestScriptVariables>()
            .get(Q, crate::QuestScriptVariables::property_key("Fired"))
    };
    crate::script_update_system(&world, 2.0, Some(0.0));
    assert_eq!(fired(&world), None, "timer 8 was cancelled, 7 is not due");
    crate::script_update_system(&world, 3.5, Some(0.02));
    assert!(crate::fragment::reference_enabled(&world, MARKER), "OnTimer(7)");
    assert_eq!(fired(&world), Some(1.0));
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 10, "0.02 h < the 0.033 h floor");
    crate::script_update_system(&world, 0.1, Some(0.04));
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
    crate::script_update_system(&world, 10.0, Some(1.0));
    assert_eq!(fired(&world), Some(1.0), "a timer fires once");
}

/// Another quest's member reads by its declared type — here an object
/// script variable the other quest's script holds — not as a number.
#[test]
fn another_quests_members_read_by_their_declared_type() {
    const OTHER: QuestFormId = QuestFormId(0x0001_0F20);
    let other_script = parsed_source(
        "ScriptName VmOtherScript extends Quest\n\
         ObjectReference Stored\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmQuestMembers extends Quest\n\
         Quest Property Other Auto\n\
         Function Fragment_1()\n\
           VmOtherScript o = Other as VmOtherScript\n\
           o.Stored.Disable()\n\
         EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("vmotherscript".to_owned(), Arc::clone(&other_script))].into();
    let library = Installed(&scripts);
    let needed = check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).expect("runs");
    let mut world = world_with_fragment(script);
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(GUARD),
    });
    world.insert_resource(pool);
    let guard = world.spawn();
    world.insert(guard, FormIdComponent(id));
    let other_vmad = ScriptInstanceData {
        version: 5,
        object_format: 2,
        scripts: vec![ScriptInstance {
            name: "VmOtherScript".into(),
            status: 0,
            properties: Vec::new(),
        }],
    };
    world.insert_resource(crate::QuestScriptInstances(Arc::new([(OTHER, other_vmad)].into())));
    world.resource_mut::<crate::QuestScriptObjects>().set(
        OTHER,
        crate::QuestScriptVariables::property_key("Stored"),
        crate::ScriptObject::Form(GUARD),
    );
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for needed in needed {
            fragments.insert_vm_script(needed);
        }
        let mut vmad = vmad();
        vmad.scripts[0].properties.push(ScriptProperty {
            name: "Other".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: OTHER.0,
                alias: -1,
            },
        });
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, GUARD));
}

/// `ScriptName.Fn()` calls a global function of another script (no `Self`);
/// `Game.GetFormFromFile` resolves a plugin-local id through the load order,
/// its top byte unused.
#[test]
fn global_functions_and_forms_from_files() {
    let util = parsed_source(
        "ScriptName VmUtil\n\
         Int Function Twice(Int n) Global\n Return n * 2\n EndFunction\n\
         Function DisableIt(ObjectReference akRef) Global\n akRef.Disable()\n EndFunction\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmGlobals2 extends Quest\n\
         Function Fragment_1()\n\
           If VmUtil.Twice(3) == 6\n\
             VmUtil.DisableIt(Game.GetFormFromFile(0x41010F01, \"Skyrim.esm\") as ObjectReference)\n\
           EndIf\n\
         EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("vmutil".to_owned(), Arc::clone(&util))].into();
    let library = Installed(&scripts);
    let needed = check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).expect("runs");
    let mut world = world_with_fragment(script);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for needed in needed {
            fragments.insert_vm_script(needed);
        }
    }
    world.insert_resource(crate::load_order::LoadOrderIdentity::new(vec![(
        byroredux_plugin::esm::reader::GlobalSlot::Regular(0),
        PluginId::from_filename("Skyrim.esm"),
    )]));
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(GUARD),
    });
    world.insert_resource(pool);
    let guard = world.spawn();
    world.insert(guard, FormIdComponent(id));
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(!crate::fragment::reference_enabled(&world, GUARD));
}

/// Papyrus `Int` arithmetic and comparison are exact 32-bit — beyond the 24
/// bits an `f32` holds.
#[test]
fn int_arithmetic_is_exact() {
    let add = arithmetic(BinaryOp::Add, &Value::Int(0x4101_0F00), &Value::Int(1)).unwrap();
    assert!(matches!(add, Value::Int(0x4101_0F01)));
    assert!(!Value::Int(0x4101_0F01).equals(&Value::Int(0x4101_0F00)));
    let lt = binary(BinaryOp::Lt, &Value::Int(0x4101_0F00), &Value::Int(0x4101_0F01)).unwrap();
    assert!(matches!(lt, Value::Bool(true)));
}

/// Fallout 4 structs and array methods: `new Struct`, members read and
/// written, `Add`, `FindStruct`, `Remove`, `Length`, an array-of-struct
/// property from the VMAD.
#[test]
fn structs_and_array_methods_run() {
    let script = parsed_source(
        "ScriptName QF_VmStructs extends Quest\n\
         Struct Entry\n Int Id\n ObjectReference Target\n EndStruct\n\
         ObjectReference Property Marker Auto\n\
         Entry[] Property Rows Auto\n\
         Function Fragment_1()\n\
           Entry[] entries = new Entry[0]\n\
           Entry first = new Entry\n\
           first.Id = 3\n\
           first.Target = Marker\n\
           entries.Add(first)\n\
           Entry second = new Entry\n\
           second.Id = 7\n\
           entries.Add(second)\n\
           Int i = entries.FindStruct(\"Id\", 3)\n\
           If i == 0 && entries.FindStruct(\"Id\", 9) < 0 && entries.Length == 2\n\
             entries[i].Target.Enable()\n\
             entries.Remove(0)\n\
             If entries.Length == 1 && entries[0].Id == 7 && Rows[0].Id == 11\n\
               Self.SetStage(20)\n\
             EndIf\n\
           EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        let mut vmad = vmad();
        vmad.scripts[0].properties.push(ScriptProperty {
            name: "Rows".into(),
            status: 1,
            value: PropertyValue::Array(vec![PropertyValue::Struct(vec![
                byroredux_plugin::esm::records::script_instance::StructMember {
                    name: "Id".into(),
                    flags: 1,
                    value: PropertyValue::Int32(11),
                },
            ])]),
        });
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(crate::fragment::reference_enabled(&world, MARKER), "entries[0].Target.Enable()");
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
    let _ = &mut world;
}
