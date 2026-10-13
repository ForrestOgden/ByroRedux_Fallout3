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

/// #5681 — a `Utility.Wait` past the shared cap stops the run instead of
/// parking an immortal continuation. The literal arm is the value the
/// lowerer already declines; the computed arm is a run-time number only
/// the VM sees. Both die with nothing queued, and the code after the wait
/// never runs.
#[test]
fn waits_past_the_shared_cap_stop_the_run() {
    for (name, source) in [
        (
            "QF_VmWaitCapLiteral",
            "ScriptName QF_VmWaitCapLiteral extends Quest\n\
             Function Fragment_1()\n\
               Utility.Wait(86401.0)\n\
               Self.SetStage(20)\n\
             EndFunction\n",
        ),
        (
            "QF_VmWaitCapComputed",
            "ScriptName QF_VmWaitCapComputed extends Quest\n\
             Function Fragment_1()\n\
               Float secs = 864010.0\n\
               Utility.Wait(secs)\n\
               Self.SetStage(20)\n\
             EndFunction\n",
        ),
    ] {
        let (script, errors) = byroredux_papyrus::parse_script(source).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        let script = Arc::new(script);
        assert!(
            check_function(Arc::clone(&script), "Fragment_1", None).is_some(),
            "the check still takes the handler; the cap is a run-time decline"
        );
        let mut world = World::new();
        crate::register(&mut world);
        let player = world.spawn();
        world.insert_resource(PapyrusPlayerEntity(player));
        world.insert_resource(QuestStageState::default());
        {
            let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
            fragments.insert_vmad(Q, vmad());
            fragments.insert_vm_script(Arc::clone(&script));
            fragments.insert(
                Q,
                10,
                vec![Effect::Interpret {
                    script: name.into(),
                    function: "Fragment_1".into(),
                    args: Vec::new(),
                    resume: None,
                }],
            );
        }
        world.resource_mut::<QuestStageState>().set_stage(Q, 10);
        crate::fragment::quest_fragment_dispatch_system(&world);
        assert_eq!(
            world.resource::<QuestStageState>().get_stage(Q),
            10,
            "{name}: the wait must stop the run, not run the code behind it"
        );
        assert!(
            world.resource::<crate::FragmentExecutionQueue>().is_empty(),
            "{name}: an over-cap wait must not park a continuation"
        );
        // No amount of time satisfies it: the queue stays empty forever.
        crate::fragment::fragment_continuation_system(&world, 1.0e30);
        assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 10);
    }
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

/// A package's Begin / Change fragments run with `akActor` bound to the
/// actor; a switch of package runs the old one's Change and the new one's
/// Begin.
#[test]
fn package_fragments_run_on_transitions() {
    use byroredux_plugin::esm::records::script_instance::PackageFragmentEvent;
    const PACK_A: u32 = 0x0001_0F30;
    const PACK_B: u32 = 0x0001_0F31;
    const ACTOR: u32 = 0x0001_0F32;
    let a = parsed_source(
        "ScriptName PF_A extends Package\n\
         Function Fragment_0(Actor akActor)\n akActor.Disable()\n EndFunction\n\
         Function Fragment_1(Actor akActor)\n Self.SetStage(30)\n EndFunction\n",
    );
    let b = parsed_source(
        "ScriptName PF_B extends Package\n\
         Function Fragment_0(Actor akActor)\n akActor.Enable()\n EndFunction\n",
    );
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(ACTOR),
    });
    world.insert_resource(pool);
    let actor = world.spawn();
    world.insert(actor, FormIdComponent(id));
    let installed = std::cell::RefCell::new(Vec::new());
    let library: HashMap<String, Arc<Script>> = HashMap::new();
    let library = Installed(&library);
    for (package, script, bindings) in [
        (
            PACK_A,
            a,
            vec![(PackageFragmentEvent::Begin, "Fragment_0"), (PackageFragmentEvent::Change, "Fragment_1")],
        ),
        (PACK_B, b, vec![(PackageFragmentEvent::Begin, "Fragment_0")]),
    ] {
        let mut fragments = world.resource_mut::<crate::PackageFragments>();
        let n = crate::populate_package_fragments(&mut fragments, package, None, script, &bindings, &library, &mut |s| {
            installed.borrow_mut().push(s)
        });
        assert_eq!(n, bindings.len());
    }
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for script in installed.into_inner() {
            fragments.insert_vm_script(script);
        }
    }
    crate::report_package_transition(&world, actor, None, Some(PACK_A));
    crate::package_fragment_system(&world, 0.0);
    assert!(!crate::fragment::reference_enabled(&world, ACTOR), "A began: akActor disabled");
    crate::report_package_transition(&world, actor, Some(PACK_A), Some(PACK_B));
    crate::package_fragment_system(&world, 0.0);
    assert!(crate::fragment::reference_enabled(&world, ACTOR), "B began: akActor enabled");
}

/// `Math` natives, `Utility.GetCurrentGameTime` (days from the host's game
/// clock) and `Game.AddAchievement` (recorded once).
#[test]
fn math_game_time_and_achievements() {
    let script = parsed_source(
        "ScriptName QF_VmMath extends Quest\n\
         Function Fragment_1()\n\
           If Math.Floor(Math.Min(7.9, 12.0)) == 7 && Math.Ceiling(2.1) == 3 && Math.Pow(2.0, 8.0) == 256.0\n\
             If Math.Abs(-5.0) == 5.0 && Utility.GetCurrentGameTime() > 1.4\n\
               Game.AddAchievement(9)\n\
               Game.AddAchievement(9)\n\
               Self.SetStage(20)\n\
             EndIf\n\
           EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let world = world_with_fragment(script);
    // 36 game hours = day 1.5.
    crate::script_update_system(&world, 0.0, Some(36.0));
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
    assert!(world.resource::<crate::Achievements>().has(9));
}

/// `Game.GetForm` answers a loaded plugin's id and None for an unloaded
/// one; `Delete()` flags a created reference like `DeleteWhenAble`.
#[test]
fn get_form_validates_the_plugin_and_delete_flags_created_references() {
    let script = parsed_source(
        "ScriptName QF_VmGetForm extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Function Fragment_1()\n\
           If Game.GetForm(0x00010F02) as Bool && !(Game.GetForm(0x7A010F02) as Bool)\n\
             Marker.Enable()\n\
           EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::load_order::LoadOrderIdentity::new(vec![(
        byroredux_plugin::esm::reader::GlobalSlot::Regular(0),
        PluginId::from_filename("Skyrim.esm"),
    )]));
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert!(crate::fragment::reference_enabled(&world, MARKER));
    let delete = parsed_source(
        "ScriptName QF_VmDelete extends Quest\n\
         ObjectReference Property Marker Auto\n\
         Function Fragment_1()\n Marker.Delete()\n EndFunction\n",
    );
    assert_eq!(check_function_reason(delete, "Fragment_1", None).err(), None);
}

/// `SetPlayerTeammate` / `SetDoingFavor` set the saved follower flags,
/// `IsPlayerTeammate` / `IsDoingFavor` read them.
#[test]
fn follower_flags_set_and_read() {
    const BUDDY: u32 = 0x0001_0F40;
    let script = parsed_source(
        "ScriptName QF_VmFollower extends Quest\n\
         Actor Property Buddy Auto\n\
         Function Fragment_1()\n\
           If !Buddy.IsPlayerTeammate()\n\
             Buddy.SetPlayerTeammate(True, False)\n\
             Buddy.SetDoingFavor()\n\
           EndIf\n\
           If Buddy.IsPlayerTeammate() && Buddy.IsDoingFavor()\n\
             Self.SetStage(20)\n\
           EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(BUDDY),
    });
    world.insert_resource(pool);
    let buddy = world.spawn();
    world.insert(buddy, FormIdComponent(id));
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        let mut vmad = vmad();
        vmad.scripts[0].properties.push(ScriptProperty {
            name: "Buddy".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: BUDDY,
                alias: -1,
            },
        });
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
    let flags = world.resource::<crate::FollowerState>().flags(BUDDY);
    assert!(flags.teammate && !flags.can_do_favor && flags.doing_favor);
}

// ── Session 99 — Papyrus VM per-frame cost census (`#[ignore]`) ─────────────
//
// `cargo test -p byroredux-scripting --release --lib -- --ignored --nocapture vm_cost`
//
// Everything the VM adds to the frame runs in exclusive `Update` systems
// (`reference_script_event_system`, `quest_fragment_dispatch_system`,
// `fragment_continuation_system`, `script_update_system`), so these are
// serial main-thread costs. Four questions, each a table row:
//   A. idle: N attached reference scripts, no events this frame
//   B. burst: one `OnCellLoad` delivered to all N in a single frame
//   C. polling: N scripts on `RegisterForUpdate(interval)`, 60 fps frames
//   D. runaway: one handler that loops until the 100 000-statement cap

fn vm_cost_world(n: usize, source: &str, name: &str) -> (World, Vec<byroredux_core::ecs::EntityId>) {
    let script = parsed_source(source);
    let (events, scripts) =
        crate::check_reference_script(Arc::clone(&script), &crate::DELIVERED_EVENTS, None).expect("the VM runs it");
    let mut world = World::new();
    crate::register(&mut world);
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));
    world.insert_resource(QuestStageState::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let mut refs = Vec::with_capacity(n);
    let ids: Vec<_> = (0..n)
        .map(|i| {
            pool.intern(FormIdPair {
                plugin: PluginId::from_filename("Skyrim.esm"),
                local: LocalFormId(0x0002_0000 + i as u32),
            })
        })
        .collect();
    world.insert_resource(pool);
    let instance = ScriptInstance { name: name.into(), status: 0, properties: Vec::new() };
    for id in ids {
        let r = world.spawn();
        world.insert(r, FormIdComponent(id));
        crate::attach_reference_script(&mut world, r, &instance, events.clone(), scripts.clone());
        refs.push(r);
    }
    (world, refs)
}

fn vm_cost_us<F: FnMut()>(iters: u32, mut f: F) -> f64 {
    for _ in 0..(iters / 10).max(1) {
        f();
    }
    let t = std::time::Instant::now();
    for _ in 0..iters {
        f();
    }
    t.elapsed().as_secs_f64() * 1e6 / f64::from(iters)
}

/// Put `world` in the budgeted state production has (the default allowance and
/// a continuation queue), or strip the budget to measure what ran before it.
fn census_mode(world: &mut World, budgeted: bool) {
    if budgeted {
        world.insert_resource(crate::VmFrameBudget::default());
        if world.try_resource::<crate::FragmentExecutionQueue>().is_none() {
            world.insert_resource(crate::FragmentExecutionQueue::default());
        }
    } else {
        world.remove_resource::<crate::VmFrameBudget>();
    }
}

fn census_frame(world: &World) -> f64 {
    let t = std::time::Instant::now();
    crate::reference_script_event_system(world, 0.0);
    crate::script_update_system(world, 1.0 / 60.0, Some(1.0));
    crate::fragment_continuation_system(world, 1.0 / 60.0);
    let ms = t.elapsed().as_secs_f64() * 1e3;
    crate::event_cleanup_system(world, 0.0);
    ms
}

#[test]
#[ignore = "timing census; run with --release --nocapture"]
fn vm_cost_idle_burst_polling_runaway() {
    const SRC: &str = "ScriptName VmCostScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Event OnInit()\n Count = 10\n EndEvent\n\
         Event OnLoad()\n Count += 1\n EndEvent\n\
         Event OnActivate(ObjectReference akActionRef)\n Count += 1\n EndEvent\n";
    eprintln!("\nA. idle frame (no events), µs per call of each VM system");
    eprintln!("{:>8} {:>14} {:>14} {:>14} {:>14}", "refs", "ref_events", "quest_frag", "continuation", "script_update");
    for n in [0usize, 100, 1000, 5000, 20000] {
        let (world, _refs) = vm_cost_world(n, SRC, "VmCostScript");
        let iters = if n >= 5000 { 200 } else { 2000 };
        let a = vm_cost_us(iters, || crate::reference_script_event_system(&world, 0.016));
        let b = vm_cost_us(iters, || crate::fragment::quest_fragment_dispatch_system(&world));
        let c = vm_cost_us(iters, || crate::fragment_continuation_system(&world, 0.016));
        let d = vm_cost_us(iters, || crate::script_update_system(&world, 0.016, Some(1.0)));
        eprintln!("{n:>8} {a:>14.2} {b:>14.2} {c:>14.2} {d:>14.2}");
    }

    eprintln!("\nB. burst: OnCellLoad to every attached reference in one frame");
    eprintln!("{:>8} {:>10} {:>14} {:>14} {:>8} {:>14}", "refs", "mode", "frame1_ms", "drain_ms", "frames", "worst_ms");
    for n in [1000usize, 5000] {
        for budgeted in [false, true] {
            let (mut world, refs) = vm_cost_world(n, SRC, "VmCostScript");
            census_mode(&mut world, budgeted);
            for r in &refs {
                world.insert(*r, crate::OnCellLoadEvent);
            }
            let first = census_frame(&world);
            for r in &refs {
                world.remove::<crate::OnCellLoadEvent>(*r);
            }
            let (mut drain, mut frames, mut worst) = (first, 1, first);
            while world.try_resource::<crate::FragmentExecutionQueue>().is_some_and(|q| !q.is_empty()) && frames < 10_000 {
                let ms = census_frame(&world);
                drain += ms;
                worst = worst.max(ms);
                frames += 1;
            }
            let mode = if budgeted { "budgeted" } else { "unbudgeted" };
            eprintln!("{n:>8} {mode:>10} {first:>14.3} {drain:>14.3} {frames:>8} {worst:>14.3}");
        }
    }

    eprintln!("\nC. polling: RegisterForUpdate(interval), 60 fps frames, µs per frame (all VM systems)");
    eprintln!("{:>8} {:>10} {:>10} {:>14} {:>14}", "refs", "interval", "mode", "mean_us/frame", "max_us/frame");
    for (n, interval) in [(100usize, "1.0"), (1000, "1.0"), (1000, "0.1"), (5000, "1.0"), (5000, "0.1")] {
        for budgeted in [false, true] {
            let src = format!(
                "ScriptName VmPollScript extends ObjectReference\n\
                 Int Property Count Auto\n\
                 Event OnInit()\n RegisterForUpdate({interval})\n EndEvent\n\
                 Event OnUpdate()\n Count += 1\n EndEvent\n"
            );
            let (mut world, refs) = vm_cost_world(n, &src, "VmPollScript");
            census_mode(&mut world, budgeted);
            for r in &refs {
                world.insert(*r, crate::OnCellLoadEvent);
            }
            // Settle the start-up burst before measuring steady polling.
            for _ in 0..200 {
                census_frame(&world);
                if world.try_resource::<crate::FragmentExecutionQueue>().is_none_or(|q| q.is_empty()) {
                    break;
                }
            }
            let frames = 600;
            let (mut sum, mut max) = (0.0f64, 0.0f64);
            for _ in 0..frames {
                let us = census_frame(&world) * 1e3;
                sum += us;
                max = max.max(us);
            }
            let mode = if budgeted { "budgeted" } else { "unbudgeted" };
            eprintln!("{n:>8} {interval:>10} {mode:>10} {:>14.2} {max:>14.2}", sum / f64::from(frames));
        }
    }

    eprintln!("\nD. runaway handlers (100000-statement cap each), OnActivate");
    eprintln!("{:>8} {:>10} {:>14} {:>14} {:>8} {:>14}", "handlers", "mode", "frame1_ms", "total_ms", "frames", "worst_ms");
    for handlers in [1usize, 5] {
        for budgeted in [false, true] {
            let (mut world, refs) = vm_cost_world(handlers, RUNAWAY, "VmRunawayScript");
            census_mode(&mut world, budgeted);
            activate_all(&mut world, &refs);
            let first = census_frame(&world);
            for r in &refs {
                world.remove::<crate::ActivateEvent>(*r);
            }
            let (mut total, mut frames, mut worst) = (first, 1, first);
            while world.try_resource::<crate::FragmentExecutionQueue>().is_some_and(|q| !q.is_empty()) && frames < 10_000 {
                let ms = census_frame(&world);
                total += ms;
                worst = worst.max(ms);
                frames += 1;
            }
            let mode = if budgeted { "budgeted" } else { "unbudgeted" };
            eprintln!("{handlers:>8} {mode:>10} {first:>14.3} {total:>14.3} {frames:>8} {worst:>14.3}");
        }
    }
}

// ── The frame budget (#5763) ─────────────────────────────────────────────────

fn budgeted(world: &mut World, limit: usize) {
    world.insert_resource(crate::VmFrameBudget::with_limit(limit));
    if world.try_resource::<crate::FragmentExecutionQueue>().is_none() {
        world.insert_resource(crate::FragmentExecutionQueue::default());
    }
}

/// A new frame as the schedule sees it: the end-of-frame cleanup renews the
/// allowance, the continuation system runs what is waiting.
fn next_frame(world: &World) {
    crate::event_cleanup_system(world, 0.0);
    crate::fragment_continuation_system(world, 1.0 / 60.0);
}

fn count_of(world: &World, index: usize) -> Option<f32> {
    world
        .resource::<crate::QuestScriptVariables>()
        .get(QuestFormId(0x0002_0000 + index as u32), crate::QuestScriptVariables::property_key("Count"))
}

const RUNAWAY: &str = "ScriptName VmRunawayScript extends ObjectReference\n\
     Int Property Count Auto\n\
     Event OnActivate(ObjectReference akActionRef)\n\
       While Count >= 0\n Count += 1\n EndWhile\n\
     EndEvent\n";

fn activate_all(world: &mut World, refs: &[byroredux_core::ecs::EntityId]) {
    let player = world.resource::<PapyrusPlayerEntity>().0;
    for r in refs {
        world.insert(*r, crate::ActivateEvent { activator: player });
    }
}

/// A run the budget cuts short suspends between two statements and resumes at
/// the next one: the loop runs exactly its iterations, none twice, none lost.
#[test]
fn a_run_the_budget_cuts_short_resumes_where_it_stopped() {
    let (mut world, refs) = vm_cost_world(
        1,
        "ScriptName VmCountScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Int Property Done Auto\n\
         Event OnActivate(ObjectReference akActionRef)\n\
           While Count < 500\n Count += 1\n EndWhile\n\
           Done = 1\n\
         EndEvent\n",
        "VmCountScript",
    );
    budgeted(&mut world, 120);
    activate_all(&mut world, &refs);
    crate::reference_script_event_system(&world, 0.0);
    let after_first = count_of(&world, 0).expect("the loop ran");
    assert!(after_first > 0.0 && after_first < 500.0, "cut short mid-loop: {after_first}");
    assert_eq!(world.resource::<crate::FragmentExecutionQueue>().len(), 1, "one continuation queued");
    assert_eq!(world.resource::<crate::VmFrameBudget>().used().yielded, 1);

    let mut frames = 0;
    while !world.resource::<crate::FragmentExecutionQueue>().is_empty() {
        next_frame(&world);
        frames += 1;
        assert!(frames < 200, "the run never finished");
    }
    assert_eq!(count_of(&world, 0), Some(500.0), "every iteration ran exactly once");
    let done = world
        .resource::<crate::QuestScriptVariables>()
        .get(QuestFormId(0x0002_0000), crate::QuestScriptVariables::property_key("Done"));
    assert_eq!(done, Some(1.0), "the statement after the loop ran");
    assert!(frames > 2, "spread over frames, not finished in one: {frames}");
}

/// Several runaway handlers raised in one frame spend the frame's allowance
/// once between them: the first yields, the rest queue unstarted, and no frame
/// afterwards spends more than the allowance either. Each run is then killed at
/// the per-run cap, counted across its yields.
#[test]
fn several_runaway_handlers_stay_inside_the_frame_budget() {
    const LIMIT: usize = 1_000;
    const RUNS: usize = 5;
    let (mut world, refs) = vm_cost_world(RUNS, RUNAWAY, "VmRunawayScript");
    budgeted(&mut world, LIMIT);
    activate_all(&mut world, &refs);
    crate::reference_script_event_system(&world, 0.0);

    let used = world.resource::<crate::VmFrameBudget>().used();
    assert!(
        used.spent >= LIMIT && used.spent <= LIMIT + 4,
        "the frame spent the allowance and no more (one statement of overshoot at most): {}",
        used.spent
    );
    assert_eq!((used.yielded, used.deferred), (1, RUNS as u32 - 1), "one cut short, the rest not started");
    assert_eq!(world.resource::<crate::FragmentExecutionQueue>().len(), RUNS);

    // Frame after frame: never over the allowance; every run dies at the cap.
    let mut frames = 0;
    let mut worst = 0;
    while !world.resource::<crate::FragmentExecutionQueue>().is_empty() {
        crate::event_cleanup_system(&world, 0.0);
        crate::fragment_continuation_system(&world, 1.0 / 60.0);
        worst = worst.max(world.resource::<crate::VmFrameBudget>().used().spent);
        frames += 1;
        assert!(frames < 2_000, "the runaway handlers never ended");
    }
    assert!(worst <= LIMIT + RUNS * RUN_START_FUZZ, "a frame went over the allowance: {worst}");
    // 100 000 statements a run, two per loop iteration, whatever the slicing.
    for index in 0..RUNS {
        let count = count_of(&world, index).expect("each ran");
        assert!(
            (45_000.0..=50_001.0).contains(&count),
            "run {index} was killed at the cumulative cap, not per slice: {count}"
        );
    }
}

/// Slack for the run-start cost the first statement of each resumed slice
/// pays before the allowance is checked.
const RUN_START_FUZZ: usize = crate::fragment::RUN_START_COST + 4;

/// A wait suspends with no carried steps (the count starts over, as it always
/// did); a yield carries them.
#[test]
fn only_a_yield_carries_the_statement_count() {
    let (mut world, refs) = vm_cost_world(
        1,
        "ScriptName VmWaitScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Event OnActivate(ObjectReference akActionRef)\n\
           Count += 1\n Utility.Wait(1.0)\n Count += 1\n\
         EndEvent\n",
        "VmWaitScript",
    );
    let carried = |world: &World| {
        let queue = world.resource::<crate::FragmentExecutionQueue>();
        let pending = queue.pending.first().cloned().expect("a continuation");
        match pending.effects.first() {
            Some(Effect::Interpret { resume: Some(resume), .. }) => resume.carried_steps,
            other => panic!("not a resumed run: {other:?}"),
        }
    };
    budgeted(&mut world, 100_000);
    activate_all(&mut world, &refs);
    crate::reference_script_event_system(&world, 0.0);
    assert_eq!(carried(&world), 0, "a wait starts the count afresh");

    // The same handler, the budget gone after its first statement.
    let (mut world, refs) = vm_cost_world(
        1,
        "ScriptName VmWaitScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Event OnActivate(ObjectReference akActionRef)\n\
           Count += 1\n Count += 1\n Count += 1\n\
         EndEvent\n",
        "VmWaitScript",
    );
    budgeted(&mut world, crate::fragment::RUN_START_COST + 1);
    activate_all(&mut world, &refs);
    crate::reference_script_event_system(&world, 0.0);
    assert!(carried(&world) > 0, "a yield carries what it has used");
}

/// A function called for its value cannot hand a value back across a
/// suspension, and a frame with an array local cannot be stored: the run is
/// not cut there. It finishes in the frame it started and is charged for it,
/// so the work behind it defers.
#[test]
fn a_run_that_cannot_suspend_finishes_and_is_charged() {
    let (mut world, refs) = vm_cost_world(
        2,
        "ScriptName VmHeavyScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Int Function Heavy()\n\
           Int i = 0\n While i < 300\n i += 1\n EndWhile\n Return i\n\
         EndFunction\n\
         Event OnActivate(ObjectReference akActionRef)\n\
           Count = Heavy()\n\
         EndEvent\n",
        "VmHeavyScript",
    );
    budgeted(&mut world, crate::fragment::RUN_START_COST + 20);
    activate_all(&mut world, &refs);
    crate::reference_script_event_system(&world, 0.0);
    assert_eq!(count_of(&world, 0), Some(300.0), "the value call ran whole");
    let used = world.resource::<crate::VmFrameBudget>().used();
    assert!(used.spent > 300, "and was charged for it: {}", used.spent);
    assert_eq!(used.yielded, 0, "never suspended");
    assert_eq!(used.deferred, 1, "the second reference waits for the next frame");
    assert_eq!(count_of(&world, 1), None);
    next_frame(&world);
    assert_eq!(count_of(&world, 1), Some(300.0), "and runs on it");
}

/// A reference whose handler cannot start for want of budget is not lost: the
/// event it was raised for is delivered on a later frame, once.
#[test]
fn deferred_deliveries_run_once_on_a_later_frame() {
    let (mut world, refs) = vm_cost_world(
        50,
        "ScriptName VmBurstScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Event OnLoad()\n Count += 1\n EndEvent\n",
        "VmBurstScript",
    );
    budgeted(&mut world, 400);
    for r in &refs {
        world.insert(*r, crate::OnCellLoadEvent);
    }
    crate::reference_script_event_system(&world, 0.0);
    let ran = |world: &World| (0..50).filter(|i| count_of(world, *i).is_some()).count();
    let first = ran(&world);
    assert!(first > 0 && first < 50, "a slice of the burst ran: {first}");
    for r in &refs {
        world.remove::<crate::OnCellLoadEvent>(*r);
    }
    let mut frames = 0;
    while !world.resource::<crate::FragmentExecutionQueue>().is_empty() {
        next_frame(&world);
        frames += 1;
        assert!(frames < 100);
    }
    assert!(frames > 1, "spread over frames");
    for index in 0..50 {
        // OnInit, OnCellAttach, OnLoad and OnCellLoad are delivered for a load;
        // the script handles OnLoad (OnCellLoad is its twin), once each.
        assert!(count_of(&world, index).is_some_and(|count| count >= 1.0), "reference {index} never ran");
    }
}

// ── The reference-script index (#5763) ───────────────────────────────────────

#[test]
fn the_reference_script_index_follows_attach_and_unload() {
    const SRC: &str = "ScriptName VmIndexScript extends ObjectReference\n\
         Int Property Count Auto\n\
         Event OnLoad()\n Count += 1\n EndEvent\n";
    let (mut world, refs) = vm_cost_world(3, SRC, "VmIndexScript");
    let form = |i: usize| 0x0002_0000 + i as u32;
    for (i, r) in refs.iter().enumerate() {
        assert_eq!(crate::ReferenceScriptIndex::lookup(&world, form(i)), Some(*r));
    }
    assert_eq!(crate::ReferenceScriptIndex::lookup(&world, 0x00FF_FFFF), None, "never attached");

    // The cell unloads: the entity goes, and the reference reads as not loaded
    // with no map maintenance at all.
    world.despawn(refs[1]);
    assert_eq!(crate::ReferenceScriptIndex::lookup(&world, form(1)), None);
    assert_eq!(crate::ReferenceScriptIndex::lookup(&world, form(0)), Some(refs[0]), "the others stand");

    // It reloads as a new entity with the same FormID and the script attached
    // again: the next lookup finds that one.
    let script = parsed_source(SRC);
    let (events, scripts) = crate::check_reference_script(script, &crate::DELIVERED_EVENTS, None).expect("the VM runs it");
    let reloaded = world.spawn();
    let id = world.resource_mut::<FormIdPool>().intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(form(1)),
    });
    world.insert(reloaded, FormIdComponent(id));
    let instance = ScriptInstance { name: "VmIndexScript".into(), status: 0, properties: Vec::new() };
    crate::attach_reference_script(&mut world, reloaded, &instance, events, scripts);
    assert_eq!(crate::ReferenceScriptIndex::lookup(&world, form(1)), Some(reloaded));
}

/// A reference whose `FormIdComponent` lands after its scripts attach is
/// still found: the FormID is read when the map is rebuilt, not at attach.
#[test]
fn the_index_reads_the_formid_when_it_is_built_not_when_a_script_attaches() {
    let script = parsed_source(
        "ScriptName VmLateScript extends ObjectReference\n Event OnLoad()\n EndEvent\n",
    );
    let (events, scripts) = crate::check_reference_script(script, &crate::DELIVERED_EVENTS, None).expect("the VM runs it");
    let mut world = World::new();
    crate::register(&mut world);
    world.register::<FormIdComponent>();
    world.insert_resource(FormIdPool::default());
    let entity = world.spawn();
    let instance = ScriptInstance { name: "VmLateScript".into(), status: 0, properties: Vec::new() };
    crate::attach_reference_script(&mut world, entity, &instance, events, scripts);
    let id = world.resource_mut::<FormIdPool>().intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(0x0003_0001),
    });
    world.insert(entity, FormIdComponent(id));
    assert_eq!(crate::ReferenceScriptIndex::lookup(&world, 0x0003_0001), Some(entity));
}

/// A static native on an engine builtin class — `InputEnableLayer.
/// GetOrCreate(3)`, the FO4+/SF player-controls idiom — must be *routed*
/// through native classification, not fail the whole check as `unknown
/// name inputenablelayer`. The library stub being an engine class (native
/// functions, no bodies) is exactly what says "this names the class, not
/// a value". `GetOrCreate` itself has no effect-table entry yet, so the
/// honest outcome is a decline naming the native — which is what drives
/// the next primitive — instead of a misleading unknown-identifier error
/// that reads as a broken script.
#[test]
fn static_native_on_an_engine_class_names_the_native_not_an_unknown() {
    let input_enable_layer = parsed_source(
        "ScriptName InputEnableLayer extends Form\n\
         Function GetOrCreate(Int aiLayersToReserve = 1) Native\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmStaticNative extends Quest\n\
         Function Fragment_1()\n\
           InputEnableLayer.GetOrCreate(3)\n\
         EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> =
        [("inputenablelayer".to_owned(), input_enable_layer)].into();
    let library = Installed(&scripts);
    assert_eq!(
        check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).err(),
        Some("static native getorcreate/1".to_owned()),
        "the decline must name the unmodeled native, not report the class \
         itself as an unknown identifier"
    );
}

/// The routing must not capture an identifier that names a VALUE: a
/// property of the fragment's own script keeps its instance-call
/// semantics even when a same-named engine class exists in the library.
#[test]
fn a_value_named_like_an_engine_class_still_resolves_as_a_value() {
    let message_stub = parsed_source(
        "ScriptName Message extends Form\n\
         Function Show(Float af1 = 0, Float af2 = 0) Native\n",
    );
    let script = parsed_source(
        "ScriptName QF_VmShadowedStatic extends Quest\n\
         Message Property MyMessage Auto\n\
         Function Fragment_1()\n\
           MyMessage.Show(1.0)\n\
         EndFunction\n",
    );
    let scripts: HashMap<String, Arc<Script>> = [("message".to_owned(), message_stub)].into();
    let library = Installed(&scripts);
    assert_eq!(
        check_function_reason(Arc::clone(&script), "Fragment_1", Some(&library)).err(),
        None,
        "MyMessage is a property — the call lowers through the instance path"
    );
}

/// Distance events: `RegisterForDistanceLessThanEvent` delivers a single
/// `OnDistanceLessThan` once the pair is close, an alias counts once filled,
/// a re-registration only updates the distance, and
/// `UnregisterForDistanceEvents` cancels it.
#[test]
fn distance_events_fire_once_when_the_pair_is_close() {
    use byroredux_core::ecs::components::GlobalTransform;
    use byroredux_core::math::{Quat, Vec3};
    const FAR: u32 = 0x0001_0F50;
    const NEAR: u32 = 0x0001_0F51;
    let script = parsed_source(
        "ScriptName QF_VmDistance extends Quest\n\
         ObjectReference Property A Auto\n\
         ObjectReference Property B Auto\n\
         ObjectReference Property C Auto\n\
         Int Fired\n\
         Function Fragment_1()\n\
           RegisterForDistanceLessThanEvent(A, B, 50.0)\n\
           RegisterForDistanceLessThanEvent(B, A, 500.0)\n\
           RegisterForDistanceLessThanEvent(A, C, 50.0)\n\
           UnregisterForDistanceEvents(A, C)\n\
         EndFunction\n\
         Event OnDistanceLessThan(ObjectReference akObj1, ObjectReference akObj2, Float afDistance)\n\
           Fired += 1\n\
           If afDistance < 300.0 && akObj1 == A\n Self.SetStage(20)\n EndIf\n\
         EndEvent\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::QuestScriptVariables::default());
    world.register::<FormIdComponent>();
    world.register::<GlobalTransform>();
    let mut pool = FormIdPool::default();
    let mut spawn_at = |world: &mut World, local: u32, x: f32| {
        let id = pool.intern(FormIdPair {
            plugin: PluginId::from_filename("Skyrim.esm"),
            local: LocalFormId(local),
        });
        let entity = world.spawn();
        world.insert(entity, FormIdComponent(id));
        world.insert(entity, GlobalTransform::new(Vec3::new(x, 0.0, 0.0), Quat::IDENTITY, 1.0));
        entity
    };
    let a = spawn_at(&mut world, FAR, 0.0);
    let b = spawn_at(&mut world, NEAR, 400.0);
    world.insert_resource(pool);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        let mut vmad = vmad();
        for (name, form) in [("A", FAR), ("B", NEAR), ("C", 0x0001_0F52)] {
            vmad.scripts[0].properties.push(ScriptProperty {
                name: name.into(),
                status: 1,
                value: PropertyValue::Object {
                    form_id: form,
                    alias: -1,
                },
            });
        }
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    // (B, A) re-registered the pair at 500: one watch, not two.
    assert_eq!(world.resource::<crate::DistanceWatches>().watches().len(), 1);
    assert_eq!(world.resource::<crate::DistanceWatches>().watches()[0].distance, 500.0);
    crate::distance_event_system(&world, 0.0);
    // 400 < 500: delivered once, with the pair in its first order.
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 10, "400 units is not under 300");
    assert!(world.resource::<crate::DistanceWatches>().watches().is_empty(), "single event");
    let fired = world
        .resource::<crate::QuestScriptVariables>()
        .get(Q, crate::QuestScriptVariables::property_key("Fired"));
    assert_eq!(fired, Some(1.0));
    crate::distance_event_system(&world, 0.0);
    assert_eq!(
        world
            .resource::<crate::QuestScriptVariables>()
            .get(Q, crate::QuestScriptVariables::property_key("Fired")),
        Some(1.0)
    );
    let _ = (a, b);
}

/// An array a script variable holds is one shared value within a run and is
/// written back whole when the run ends: a later fragment sees the elements
/// the earlier one added, structs included.
#[test]
fn script_variable_arrays_persist_between_runs() {
    let script = parsed_source(
        "ScriptName QF_VmHeld extends Quest\n\
         Struct Entry\n Int Id\n EndStruct\n\
         ObjectReference Property Marker Auto\n\
         Entry[] Rows\n\
         Int[] Counts\n\
         Function Fragment_1()\n\
           Counts = new Int[0]\n\
           Counts.Add(4)\n\
           Counts.Add(9)\n\
           Rows = new Entry[0]\n\
           Entry e = new Entry\n\
           e.Id = 7\n\
           Rows.Add(e)\n\
           Rows[0].Id = 8\n\
         EndFunction\n\
         Function Fragment_2()\n\
           If Counts.Length == 2 && Counts[1] == 9 && Rows.Length == 1 && Rows[0].Id == 8\n\
             Counts.Remove(0)\n\
             Marker.Enable()\n\
           EndIf\n\
         EndFunction\n\
         Function Fragment_3()\n\
           If Counts.Length == 1 && Counts[0] == 9\n\
             Self.SetStage(30)\n\
           EndIf\n\
         EndFunction\n",
    );
    for function in ["Fragment_1", "Fragment_2", "Fragment_3"] {
        assert_eq!(check_function_reason(Arc::clone(&script), function, None).err(), None, "{function}");
    }
    let world = world_with_fragment(script);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        for (stage, function) in [(20, "Fragment_1"), (21, "Fragment_2"), (22, "Fragment_3")] {
            fragments.insert(
                Q,
                stage,
                vec![Effect::Interpret {
                    script: "QF_VmHeld".into(),
                    function: function.into(),
                    args: Vec::new(),
                    resume: None,
                }],
            );
        }
    }
    for stage in [20, 21, 22] {
        world.resource_mut::<QuestStageState>().set_stage(Q, stage);
        crate::fragment::quest_fragment_dispatch_system(&world);
    }
    assert!(crate::fragment::reference_enabled(&world, MARKER), "the second run read the first's arrays");
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 30, "and the third read the second's Remove");
}

/// A script-filled `RefCollectionAlias`: `AddRef` / `AddArray` / `RemoveRef`
/// / `RemoveAll` change it, `GetCount` / `GetAt` / `Find` read it, across
/// runs; a collection the quest fills by conditions is refused.
#[test]
fn script_filled_collection_aliases() {
    use byroredux_plugin::esm::records::{QuestAlias, QustRecord};
    const FIRST: u32 = 0x0001_0F60;
    const SECOND: u32 = 0x0001_0F61;
    let script = parsed_source(
        "ScriptName QF_VmCollection extends Quest\n\
         RefCollectionAlias Property Crowd Auto\n\
         RefCollectionAlias Property Filled Auto\n\
         ObjectReference Property A Auto\n\
         ObjectReference Property B Auto\n\
         Function Fragment_1()\n\
           Crowd.AddRef(A)\n\
           Crowd.AddRef(B)\n\
           Crowd.AddRef(A)\n\
         EndFunction\n\
         Function Fragment_2()\n\
           If Crowd.GetCount() == 2 && Crowd.Find(B) == 1 && Crowd.GetAt(0) == A\n\
             ObjectReference[] all = Crowd.GetArray()\n\
             If all.Length == 2\n\
               Crowd.DisableAll()\n\
               Crowd.RemoveRef(A)\n\
               Self.SetStage(30)\n\
             EndIf\n\
           EndIf\n\
         EndFunction\n\
         Function Fragment_3()\n\
           If Crowd.GetCount() == 1 && Crowd.GetAt(0) == B\n\
             Crowd.RemoveAll()\n\
           EndIf\n\
           Filled.GetCount()\n\
         EndFunction\n",
    );
    for function in ["Fragment_1", "Fragment_2", "Fragment_3"] {
        assert_eq!(check_function_reason(Arc::clone(&script), function, None).err(), None, "{function}");
    }
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::QuestScriptVariables::default());
    let record = QustRecord {
        form_id: Q.0,
        aliases: vec![
            QuestAlias {
                alias_id: 3,
                is_collection: true,
                ..Default::default()
            },
            QuestAlias {
                alias_id: 4,
                is_collection: true,
                match_conditions: {
                    let mut conditions = byroredux_plugin::esm::records::condition::ConditionList::default();
                    conditions.push(Default::default());
                    conditions
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    crate::scene::install_scene_quest_aliases(&mut world, [record]);
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        let mut vmad = vmad();
        let alias = |name: &str, id: i16| ScriptProperty {
            name: name.into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: Q.0,
                alias: id,
            },
        };
        let object = |name: &str, form: u32| ScriptProperty {
            name: name.into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: form,
                alias: -1,
            },
        };
        vmad.scripts[0].properties.extend([alias("Crowd", 3), alias("Filled", 4), object("A", FIRST), object("B", SECOND)]);
        fragments.insert_vmad(Q, vmad);
        for (stage, function) in [(20, "Fragment_1"), (21, "Fragment_2"), (22, "Fragment_3")] {
            fragments.insert(
                Q,
                stage,
                vec![Effect::Interpret {
                    script: "QF_VmCollection".into(),
                    function: function.into(),
                    args: Vec::new(),
                    resume: None,
                }],
            );
        }
    }
    for stage in [20, 21] {
        world.resource_mut::<QuestStageState>().set_stage(Q, stage);
        crate::fragment::quest_fragment_dispatch_system(&world);
    }
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 30, "the count, find and get-at read");
    assert_eq!(world.resource::<crate::AliasCollections>().members((Q.0, 3)), [SECOND]);
    assert!(!crate::fragment::reference_enabled(&world, FIRST) && !crate::fragment::reference_enabled(&world, SECOND), "DisableAll");
    world.resource_mut::<QuestStageState>().set_stage(Q, 22);
    crate::fragment::quest_fragment_dispatch_system(&world);
    // RemoveAll emptied it; the conditions-filled one was refused, not read as
    // empty (the run stopped there, after RemoveAll).
    assert!(world.resource::<crate::AliasCollections>().members((Q.0, 3)).is_empty());
}

/// `HasLocRefType` reads the reference's XLRT tags.
#[test]
fn has_loc_ref_type_reads_the_placement_tags() {
    const TAGGED: u32 = 0x0001_0F70;
    const BOSS: u32 = 0x0001_0F71;
    const OTHER: u32 = 0x0001_0F72;
    let script = parsed_source(
        "ScriptName QF_VmRefType extends Quest\n\
         ObjectReference Property Subject Auto\n\
         Form Property BossType Auto\n\
         Form Property OtherType Auto\n\
         Function Fragment_1()\n\
           If Subject.HasLocRefType(BossType as LocationRefType) && !Subject.HasLocRefType(OtherType as LocationRefType)\n\
             Self.SetStage(20)\n\
           EndIf\n\
         EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(TAGGED),
    });
    world.insert_resource(pool);
    let subject = world.spawn();
    world.insert(subject, FormIdComponent(id));
    world.insert(
        subject,
        crate::scene::SceneAliasCandidate {
            reference_form_id: TAGGED,
            location_ref_types: vec![BOSS],
            ..Default::default()
        },
    );
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        let mut vmad = vmad();
        for (name, form) in [("Subject", TAGGED), ("BossType", BOSS), ("OtherType", OTHER)] {
            vmad.scripts[0].properties.push(ScriptProperty {
                name: name.into(),
                status: 1,
                value: PropertyValue::Object {
                    form_id: form,
                    alias: -1,
                },
            });
        }
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(world.resource::<QuestStageState>().get_stage(Q), 20);
}

/// Custom events: a registered script's `(akSender, akArgs)` handler runs from
/// the continuation queue after the sender's run, with the array it sent; an
/// unregistered one does not. Remote events relay what a reference received.
#[test]
fn custom_and_remote_events_reach_registered_scripts() {
    const DOOR: u32 = 0x0001_0F80;
    let script = parsed_source(
        "ScriptName QF_VmEvents extends Quest\n\
         CustomEvent Ping\n\
         ObjectReference Property Marker Auto\n\
         ObjectReference Property Door Auto\n\
         Int Pings\n\
         Function Fragment_1()\n\
           RegisterForCustomEvent(Self as ScriptObject, \"qf_vmevents_Ping\")\n\
           RegisterForRemoteEvent(Door, \"OnActivate\")\n\
           Var[] args = new Var[2]\n\
           args[0] = 5\n\
           args[1] = 2.5\n\
           SendCustomEvent(\"qf_vmevents_Ping\", args)\n\
           Pings = 100\n\
         EndFunction\n\
         Event QF_VmEvents_Ping(QF_VmEvents akSender, Var[] akArgs)\n\
           Pings += 1\n\
           If akArgs.Length == 2 && (akArgs[0] as Int) == 5\n Marker.Enable()\n EndIf\n\
         EndEvent\n\
         Event ObjectReference_OnActivate(ObjectReference akSender, ObjectReference akActionRef)\n\
           Pings += 10\n\
         EndEvent\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let mut world = world_with_fragment(script);
    world.insert_resource(crate::QuestScriptVariables::default());
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let id = pool.intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(DOOR),
    });
    world.insert_resource(pool);
    let door = world.spawn();
    world.insert(door, FormIdComponent(id));
    {
        let mut fragments = world.resource_mut::<crate::QuestStageFragments>();
        let mut vmad = vmad();
        vmad.scripts[0].properties.push(ScriptProperty {
            name: "Door".into(),
            status: 1,
            value: PropertyValue::Object {
                form_id: DOOR,
                alias: -1,
            },
        });
        fragments.insert_vmad(Q, vmad);
    }
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    let pings = |world: &World| {
        world
            .resource::<crate::QuestScriptVariables>()
            .get(Q, crate::QuestScriptVariables::property_key("Pings"))
    };
    // The sender's run finished (Pings = 100) before the receiver ran.
    assert_eq!(pings(&world), Some(100.0));
    assert!(!crate::fragment::reference_enabled(&world, MARKER));
    crate::fragment::fragment_continuation_system(&world, 0.0);
    assert_eq!(pings(&world), Some(101.0), "the custom handler ran once, with the sent array");
    assert!(crate::fragment::reference_enabled(&world, MARKER));
    // The door is activated: the remote registrant hears it.
    world.insert(door, crate::ActivateEvent { activator: door });
    crate::reference_script_event_system(&world, 0.0);
    crate::fragment::fragment_continuation_system(&world, 0.0);
    assert_eq!(pings(&world), Some(111.0));
}

/// `Game.AdvanceSkill` queues the usage XP for the host's skill system.
#[test]
fn advance_skill_queues_the_usage_xp() {
    let script = parsed_source(
        "ScriptName QF_VmSkill extends Quest\n\
         Function Fragment_1()\n Game.AdvanceSkill(\"Lockpicking\", 50.0)\n EndFunction\n",
    );
    assert_eq!(check_function_reason(Arc::clone(&script), "Fragment_1", None).err(), None);
    let world = world_with_fragment(script);
    world.resource_mut::<QuestStageState>().set_stage(Q, 10);
    crate::fragment::quest_fragment_dispatch_system(&world);
    assert_eq!(
        world.resource::<crate::PendingSkillAdvances>().0,
        [("Lockpicking".to_owned(), 50.0)]
    );
}
