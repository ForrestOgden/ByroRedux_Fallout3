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
