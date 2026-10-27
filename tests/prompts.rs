use rust_magi::{
    agent::{factory::MagiCoreID, intent::Intention},
    prompt::create_prompt::{create_meeting_prompt, create_sys_prompt, create_vote_prompt},
    workflow::meeting::{MeetingMessage, MeetingRound},
};

#[test]
fn system_prompt_uses_the_selected_personality_and_preamble() {
    for (id, marker) in [
        (MagiCoreID::Balthasar, "BALTHASAR-2"),
        (MagiCoreID::Casper, "CASPER-3"),
        (MagiCoreID::Melchior, "MELCHIOR-1"),
    ] {
        let prompt = create_sys_prompt(&id, "follow the schema");
        assert!(prompt.contains(marker), "{prompt}");
        assert!(prompt.ends_with("follow the schema\n\n"), "{prompt}");
    }
}

#[test]
fn vote_prompt_maps_both_classifier_options_to_booleans() {
    let intent = Intention::Vote {
        option_a: "Ship today".into(),
        option_b: "Wait for tests".into(),
    };
    let prompt = create_vote_prompt(&intent, "Should we release?");
    for text in [
        "Should we release?",
        "`Ship today`对应true",
        "`Wait for tests`对应false",
    ] {
        assert!(prompt.contains(text), "missing {text}: {prompt}");
    }
}

#[test]
fn meeting_prompt_carries_candidate_and_public_discussion() {
    let rounds = [MeetingRound {
        candidate: None,
        messages: vec![MeetingMessage {
            id: MagiCoreID::Casper,
            message: "Start small".into(),
            proposal: "Pilot first".into(),
            agrees_with_candidate: false,
        }],
    }];
    let prompt = create_meeting_prompt("How to launch?", &rounds, Some("Pilot first"));
    for text in [
        "How to launch?",
        "Pilot first",
        "CASPER：Start small",
        "第 1 轮",
    ] {
        assert!(prompt.contains(text), "missing {text}: {prompt}");
    }
}

#[test]
fn intent_display_exposes_the_selected_mode_and_options() {
    let vote = Intention::Vote {
        option_a: "Ship".into(),
        option_b: "Wait".into(),
    };
    assert_eq!(vote.to_string(), "投票：选项1[Ship]，选项2[Wait]");

    let meeting = Intention::Meeting {
        interpretation: "Discuss release strategy".into(),
    };
    assert_eq!(meeting.to_string(), "会议：Discuss release strategy");
}
