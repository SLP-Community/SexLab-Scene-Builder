use super::animlist::AnimLine;

/// Escape text for Havok XML element bodies / attribute values.
/// No-op for typical FNIS event names (keeps byte-identical output).
fn xml_escape(s: &str) -> String {
    if !s.bytes().any(|b| matches!(b, b'&' | b'<' | b'>' | b'"')) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

struct IdGen {
    next: u32,
}

impl IdGen {
    fn new() -> Self {
        Self { next: 1 }
    }

    fn alloc(&mut self) -> String {
        let id = self.next;
        self.next += 1;
        format!("#{id:04}")
    }
}

/// Build hk_2010.2.0-r1 XML for a P+ `b`-line AnimList.
///
/// `fixed_events` must be the 11 race-specific FNIS reserved event names
/// (indices 0–10); animation events follow at 11+.
///
/// Object IDs and emission order follow FNIS for Modders output (black-box):
/// each anim reserves its state ID first; shared HeadTracking enter/exit are
/// created lazily on the first non-AO line; AO payloads are allocated after
/// the enter slot but emitted before the enter array; idles wildcard is
/// allocated before the blend effect but emitted after it; graph data is
/// allocated before variable/string data but emitted after them.
pub fn build_behavior_xml(pack_name: &str, lines: &[AnimLine], fixed_events: &[&str]) -> String {
    assert!(
        fixed_events.len() == 11,
        "FNIS fixed event table must have exactly 11 entries"
    );
    let mut ids = IdGen::new();
    let mut body = String::new();

    let root_id = ids.alloc();
    let graph_id = ids.alloc();
    let root_sm_id = ids.alloc();
    let idles_state_id = ids.alloc();
    let idles_sm_id = ids.alloc();

    let mut shared_enter_id: Option<String> = None;
    let mut shared_exit_id: Option<String> = None;
    let mut state_ids = Vec::new();
    let mut payload_cache: Vec<(String, String)> = Vec::new(); // (ao name, object id)

    for (state_index, line) in lines.iter().enumerate() {
        // FNIS reserves the state object ID before any of that anim's children.
        let state_id = ids.alloc();
        state_ids.push(state_id.clone());

        let mode = if line.is_acyclic() {
            "MODE_SINGLE_PLAY"
        } else {
            "MODE_LOOPING"
        };
        let clip_name = xml_escape(&format!("{}_{}", pack_name, line.file_stem()));
        let anim_path = xml_escape(&format!(r"Animations\{}\{}", pack_name, line.file));

        if line.has_anim_objects() {
            // enter ID reserved before new payloads (enter_id < first new payload).
            let enter_id = ids.alloc();
            let mut new_payloads: Vec<(String, String)> = Vec::new();
            let mut enter_events: Vec<(i32, Option<String>)> = vec![(4, None)];
            for ao in &line.anim_objects {
                let payload_id = if let Some((_, id)) = payload_cache.iter().find(|(n, _)| n == ao) {
                    id.clone()
                } else {
                    let id = ids.alloc();
                    payload_cache.push((ao.clone(), id.clone()));
                    new_payloads.push((id.clone(), ao.clone()));
                    id
                };
                enter_events.push((2, Some(payload_id.clone())));
                enter_events.push((3, Some(payload_id)));
            }
            let exit_id = ids.alloc();

            let triggers_ref = if line.is_acyclic() || line.has_tn() {
                let trig_id = ids.alloc();
                // Emit triggers after enter/exit, before clip (md+a packs).
                for (pid, name) in &new_payloads {
                    body.push_str(&string_payload(pid, &xml_escape(name)));
                }
                body.push_str(&event_prop_array(&enter_id, &enter_events));
                body.push_str(&event_prop_array(&exit_id, &[(5, None), (1, None)]));
                body.push_str(&empty_clip_trigger_array(&trig_id));
                trig_id
            } else {
                for (pid, name) in &new_payloads {
                    body.push_str(&string_payload(pid, &xml_escape(name)));
                }
                body.push_str(&event_prop_array(&enter_id, &enter_events));
                body.push_str(&event_prop_array(&exit_id, &[(5, None), (1, None)]));
                "null".to_string()
            };

            let clip_id = ids.alloc();
            body.push_str(&clip_generator(
                &clip_id,
                &clip_name,
                &anim_path,
                &triggers_ref,
                mode,
            ));
            body.push_str(&state_info(
                &state_id,
                &enter_id,
                &exit_id,
                &clip_id,
                &xml_escape(&line.event),
                state_index as i32,
            ));
        } else {
            let (enter_id, exit_id, emit_shared) = match (&shared_enter_id, &shared_exit_id) {
                (Some(e), Some(x)) => (e.clone(), x.clone(), false),
                _ => {
                    let e = ids.alloc();
                    let x = ids.alloc();
                    shared_enter_id = Some(e.clone());
                    shared_exit_id = Some(x.clone());
                    (e, x, true)
                }
            };

            let triggers_ref = if line.is_acyclic() || line.has_tn() {
                let trig_id = ids.alloc();
                if emit_shared {
                    body.push_str(&event_prop_array(&enter_id, &[(4, None)]));
                    body.push_str(&event_prop_array(&exit_id, &[(5, None)]));
                }
                body.push_str(&empty_clip_trigger_array(&trig_id));
                trig_id
            } else {
                if emit_shared {
                    body.push_str(&event_prop_array(&enter_id, &[(4, None)]));
                    body.push_str(&event_prop_array(&exit_id, &[(5, None)]));
                }
                "null".to_string()
            };

            let clip_id = ids.alloc();
            body.push_str(&clip_generator(
                &clip_id,
                &clip_name,
                &anim_path,
                &triggers_ref,
                mode,
            ));
            body.push_str(&state_info(
                &state_id,
                &enter_id,
                &exit_id,
                &clip_id,
                &xml_escape(&line.event),
                state_index as i32,
            ));
        }
    }

    // Wildcard transition array ID before blend; emit blend then wildcard.
    let wildcard_id = ids.alloc();
    let blend_id = ids.alloc();
    body.push_str(&blend_transition(&blend_id));

    let mut transitions = String::new();
    for (i, _) in lines.iter().enumerate() {
        let event_id = fixed_events.len() as i32 + i as i32;
        transitions.push_str(&wildcard_transition(&blend_id, event_id, i as i32));
    }
    body.push_str(&transition_array(&wildcard_id, lines.len(), &transitions));

    let states_list = state_ids.join(" ");
    body.push_str(&state_machine(
        &idles_sm_id,
        "FNISIdlesBehavior",
        1, // FNIS always starts Idles SM at state 1 (observed across reference packs)
        &states_list,
        lines.len(),
        &wildcard_id,
    ));

    body.push_str(&state_info(
        &idles_state_id,
        "null",
        "null",
        &idles_sm_id,
        "FNISIdles",
        0,
    ));

    let root_wildcard_id = ids.alloc();
    body.push_str(&transition_array(&root_wildcard_id, 0, ""));

    body.push_str(&state_machine(
        &root_sm_id,
        "FNIS_RootBehavior",
        0,
        &idles_state_id,
        1,
        &root_wildcard_id,
    ));

    // Graph data ID before var/string; emit var, string, then graph data.
    let graph_data_id = ids.alloc();
    let var_values_id = ids.alloc();
    let string_data_id = ids.alloc();

    body.push_str(&variable_value_set(&var_values_id));

    let mut event_names = fixed_events
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    for line in lines {
        event_names.push(line.event.clone());
    }
    body.push_str(&string_data(&string_data_id, &event_names));
    body.push_str(&graph_data(
        &graph_data_id,
        &var_values_id,
        &string_data_id,
        event_names.len(),
    ));

    body.push_str(&behavior_graph(&graph_id, &root_sm_id, &graph_data_id));
    body.push_str(&root_container(&root_id, &graph_id));

    format!(
        r#"<?xml version="1.0" encoding="ascii"?>
<hkpackfile classversion="8" contentsversion="hk_2010.2.0-r1" toplevelobject="{root_id}">

	<hksection name="__data__">

{body}
	</hksection>

</hkpackfile>
"#
    )
}

fn event_prop_array(id: &str, events: &[(i32, Option<String>)]) -> String {
    let mut items = String::new();
    for (eid, payload) in events {
        let p = payload
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("null");
        items.push_str(&format!(
            r#"
				<hkobject>
					<hkparam name="id">{eid}</hkparam>
					<hkparam name="payload">{p}</hkparam>
				</hkobject>"#
        ));
    }
    format!(
        r#"
		<hkobject name="{id}" class="hkbStateMachineEventPropertyArray" signature="0xb07b4388">
			<hkparam name="events" numelements="{}">{items}
			</hkparam>
		</hkobject>
"#,
        events.len()
    )
}

fn string_payload(id: &str, data: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbStringEventPayload" signature="0xed04256a">
			<hkparam name="data">{data}</hkparam>
		</hkobject>
"#
    )
}

fn empty_clip_trigger_array(id: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbClipTriggerArray" signature="0x59c23a0f">
			<hkparam name="triggers" numelements="0"></hkparam>
		</hkobject>
"#
    )
}

fn clip_generator(id: &str, name: &str, anim_path: &str, triggers: &str, mode: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbClipGenerator" signature="0x333b85b9">
			<hkparam name="variableBindingSet">null</hkparam>
			<hkparam name="userData">0</hkparam>
			<hkparam name="name">{name}</hkparam>
			<hkparam name="animationName">{anim_path}</hkparam>
			<hkparam name="triggers">{triggers}</hkparam>
			<hkparam name="cropStartAmountLocalTime">0.000000</hkparam>
			<hkparam name="cropEndAmountLocalTime">0.000000</hkparam>
			<hkparam name="startTime">0.000000</hkparam>
			<hkparam name="playbackSpeed">1.000000</hkparam>
			<hkparam name="enforcedDuration">0.000000</hkparam>
			<hkparam name="userControlledTimeFraction">0.000000</hkparam>
			<hkparam name="animationBindingIndex">-1</hkparam>
			<hkparam name="mode">{mode}</hkparam>
			<hkparam name="flags">0</hkparam>
		</hkobject>
"#
    )
}

fn state_info(
    id: &str,
    enter: &str,
    exit: &str,
    generator: &str,
    name: &str,
    state_id: i32,
) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbStateMachineStateInfo" signature="0xed7f9d0">
			<hkparam name="variableBindingSet">null</hkparam>
			<hkparam name="listeners" numelements="0"></hkparam>
			<hkparam name="enterNotifyEvents">{enter}</hkparam>
			<hkparam name="exitNotifyEvents">{exit}</hkparam>
			<hkparam name="transitions">null</hkparam>
			<hkparam name="generator">{generator}</hkparam>
			<hkparam name="name">{name}</hkparam>
			<hkparam name="stateId">{state_id}</hkparam>
			<hkparam name="probability">1.000000</hkparam>
			<hkparam name="enable">true</hkparam>
		</hkobject>
"#
    )
}

fn blend_transition(id: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbBlendingTransitionEffect" signature="0xfd8584fe">
			<hkparam name="variableBindingSet">null</hkparam>
			<hkparam name="userData">0</hkparam>
			<hkparam name="name">FNIS_06sec_BlendTransition</hkparam>
			<hkparam name="selfTransitionMode">SELF_TRANSITION_MODE_CONTINUE_IF_CYCLIC_BLEND_IF_ACYCLIC</hkparam>
			<hkparam name="eventMode">EVENT_MODE_PROCESS_ALL</hkparam>
			<hkparam name="duration">0.600000</hkparam>
			<hkparam name="toGeneratorStartTimeFraction">0.000000</hkparam>
			<hkparam name="flags">FLAG_IGNORE_FROM_WORLD_FROM_MODEL</hkparam>
			<hkparam name="endMode">END_MODE_NONE</hkparam>
			<hkparam name="blendCurve">BLEND_CURVE_SMOOTH</hkparam>
		</hkobject>
"#
    )
}

fn wildcard_transition(blend_id: &str, event_id: i32, to_state: i32) -> String {
    format!(
        r#"
				<hkobject>
					<hkparam name="triggerInterval">
						<hkobject>
							<hkparam name="enterEventId">-1</hkparam>
							<hkparam name="exitEventId">-1</hkparam>
							<hkparam name="enterTime">0.000000</hkparam>
							<hkparam name="exitTime">0.000000</hkparam>
						</hkobject>
					</hkparam>
					<hkparam name="initiateInterval">
						<hkobject>
							<hkparam name="enterEventId">-1</hkparam>
							<hkparam name="exitEventId">-1</hkparam>
							<hkparam name="enterTime">0.000000</hkparam>
							<hkparam name="exitTime">0.000000</hkparam>
						</hkobject>
					</hkparam>
					<hkparam name="transition">{blend_id}</hkparam>
					<hkparam name="condition">null</hkparam>
					<hkparam name="eventId">{event_id}</hkparam>
					<hkparam name="toStateId">{to_state}</hkparam>
					<hkparam name="fromNestedStateId">0</hkparam>
					<hkparam name="toNestedStateId">0</hkparam>
					<hkparam name="priority">0</hkparam>
					<hkparam name="flags">FLAG_DISABLE_CONDITION|FLAG_IS_GLOBAL_WILDCARD|FLAG_IS_LOCAL_WILDCARD</hkparam>
				</hkobject>"#
    )
}

fn transition_array(id: &str, count: usize, items: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbStateMachineTransitionInfoArray" signature="0xe397b11e">
			<hkparam name="transitions" numelements="{count}">{items}
			</hkparam>
		</hkobject>
"#
    )
}

fn state_machine(
    id: &str,
    name: &str,
    start_state_id: i32,
    states: &str,
    state_count: usize,
    wildcard: &str,
) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbStateMachine" signature="0x816c1dcb">
			<hkparam name="variableBindingSet">null</hkparam>
			<hkparam name="userData">0</hkparam>
			<hkparam name="name">{name}</hkparam>
			<hkparam name="eventToSendWhenStateOrTransitionChanges">
				<hkobject>
					<hkparam name="id">-1</hkparam>
					<hkparam name="payload">null</hkparam>
				</hkobject>
			</hkparam>
			<hkparam name="startStateChooser">null</hkparam>
			<hkparam name="startStateId">{start_state_id}</hkparam>
			<hkparam name="returnToPreviousStateEventId">-1</hkparam>
			<hkparam name="randomTransitionEventId">-1</hkparam>
			<hkparam name="transitionToNextHigherStateEventId">-1</hkparam>
			<hkparam name="transitionToNextLowerStateEventId">-1</hkparam>
			<hkparam name="syncVariableIndex">-1</hkparam>
			<hkparam name="wrapAroundStateId">false</hkparam>
			<hkparam name="maxSimultaneousTransitions">32</hkparam>
			<hkparam name="startStateMode">START_STATE_MODE_DEFAULT</hkparam>
			<hkparam name="selfTransitionMode">SELF_TRANSITION_MODE_NO_TRANSITION</hkparam>
			<hkparam name="states" numelements="{state_count}">
				{states}
			</hkparam>
			<hkparam name="wildcardTransitions">{wildcard}</hkparam>
		</hkobject>
"#
    )
}

fn variable_value_set(id: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbVariableValueSet" signature="0x27812d8d">
			<hkparam name="wordVariableValues" numelements="2">
				<hkobject>
					<hkparam name="value">0</hkparam>
				</hkobject>
				<hkobject>
					<hkparam name="value">0</hkparam>
				</hkobject>
			</hkparam>
			<hkparam name="quadVariableValues" numelements="0"></hkparam>
			<hkparam name="variantVariableValues" numelements="0"></hkparam>
		</hkobject>
"#
    )
}

fn string_data(id: &str, event_names: &[String]) -> String {
    let events: String = event_names
        .iter()
        .map(|e| format!("\n\t\t\t\t<hkcstring>{}</hkcstring>", xml_escape(e)))
        .collect();
    format!(
        r#"
		<hkobject name="{id}" class="hkbBehaviorGraphStringData" signature="0xc713064e">
			<hkparam name="eventNames" numelements="{}">{events}
			</hkparam>
			<hkparam name="attributeNames" numelements="1">
				<hkcstring>AttrWM</hkcstring>
			</hkparam>
			<hkparam name="variableNames" numelements="2">
				<hkcstring>bAnimationDriven</hkcstring>
				<hkcstring>IsFNIS</hkcstring>
			</hkparam>
			<hkparam name="characterPropertyNames" numelements="0"></hkparam>
		</hkobject>
"#,
        event_names.len()
    )
}

fn graph_data(id: &str, var_values: &str, string_data: &str, event_count: usize) -> String {
    let mut event_infos = String::new();
    for _ in 0..event_count {
        event_infos.push_str(
            r#"
				<hkobject>
					<hkparam name="flags">0</hkparam>
				</hkobject>"#,
        );
    }
    format!(
        r#"
		<hkobject name="{id}" class="hkbBehaviorGraphData" signature="0x95aca5d">
			<hkparam name="attributeDefaults" numelements="1">
				0.000000
			</hkparam>
			<hkparam name="variableInfos" numelements="2">
				<hkobject>
					<hkparam name="role">
						<hkobject>
							<hkparam name="role">ROLE_DEFAULT</hkparam>
							<hkparam name="flags">0</hkparam>
						</hkobject>
					</hkparam>
					<hkparam name="type">VARIABLE_TYPE_BOOL</hkparam>
				</hkobject>
				<hkobject>
					<hkparam name="role">
						<hkobject>
							<hkparam name="role">ROLE_DEFAULT</hkparam>
							<hkparam name="flags">0</hkparam>
						</hkobject>
					</hkparam>
					<hkparam name="type">VARIABLE_TYPE_INT32</hkparam>
				</hkobject>
			</hkparam>
			<hkparam name="characterPropertyInfos" numelements="0"></hkparam>
			<hkparam name="eventInfos" numelements="{event_count}">{event_infos}
			</hkparam>
			<hkparam name="wordMinVariableValues" numelements="0"></hkparam>
			<hkparam name="wordMaxVariableValues" numelements="0"></hkparam>
			<hkparam name="variableInitialValues">{var_values}</hkparam>
			<hkparam name="stringData">{string_data}</hkparam>
		</hkobject>
"#
    )
}

fn behavior_graph(id: &str, root_gen: &str, data: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkbBehaviorGraph" signature="0xb1218f86">
			<hkparam name="variableBindingSet">null</hkparam>
			<hkparam name="userData">0</hkparam>
			<hkparam name="name">FNISBehavior.hkb</hkparam>
			<hkparam name="variableMode">VARIABLE_MODE_DISCARD_WHEN_INACTIVE</hkparam>
			<hkparam name="rootGenerator">{root_gen}</hkparam>
			<hkparam name="data">{data}</hkparam>
		</hkobject>
"#
    )
}

fn root_container(id: &str, graph: &str) -> String {
    format!(
        r#"
		<hkobject name="{id}" class="hkRootLevelContainer" signature="0x2772c11e">
			<hkparam name="namedVariants" numelements="1">
				<hkobject>
					<hkparam name="name">hkbBehaviorGraph</hkparam>
					<hkparam name="className">hkbBehaviorGraph</hkparam>
					<hkparam name="variant">{graph}</hkparam>
				</hkobject>
			</hkparam>
		</hkobject>
"#
    )
}
