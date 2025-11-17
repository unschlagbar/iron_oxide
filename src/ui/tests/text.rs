use std::{cell::RefCell, rc::Rc};

use crate::{
    graphics::formats::RGBA,
    ui::{
        Container, OutArea, ScrollPanel, Text, UiState,
        UiUnit::*,
        tests::test_env::TestApp,
        text_layout::{OverflowWrap, TextLayout, TextOverflow, WhiteSpace},
    },
};

#[test]
pub fn text_test() {
    let mut global_ui = Rc::new(RefCell::new(UiState::create(true)));
    let mut ui = global_ui.borrow_mut();

    let root = ui.add_element(
        Container {
            color: RGBA::ZERO,
            width: Relative(1.0),
            height: Relative(1.0),
            ..Default::default()
        },
        "text_test_root",
    );

    let content = ui
        .add_child_to(
            Container {
                color: RGBA::rgb(30, 30, 30),
                width: Fill,
                height: Fill,
                padding: OutArea::new(6.0),
                ..Default::default()
            },
            "content",
            root,
        )
        .unwrap();

    let scroll = ui
        .add_child_to(
            ScrollPanel {
                padding: OutArea::new(4.0),
                ..Default::default()
            },
            "scroll",
            content,
        )
        .unwrap();

    // A helper to add a sample text box with label
    let add_sample = |ui: &mut UiState, parent: u32, label: &str, txt: Text| {
        // label
        ui.add_child_to(
            Text {
                text: label.to_string(),
                color: RGBA::rgb(200, 200, 200),
                layout: TextLayout {
                    font_size: 8.0,
                    ..Default::default()
                },
                ..Default::default()
            },
            "",
            parent,
        )
        .unwrap();

        // box with sample text
        let box_id = ui
            .add_child_to(
                Container {
                    color: RGBA::rgb(50, 50, 50),
                    width: Fill,
                    height: Auto,
                    padding: OutArea::new(6.0),
                    margin: OutArea::new(4.0),
                    ..Default::default()
                },
                "",
                parent,
            )
            .unwrap();

        ui.add_child_to(txt, "", box_id).unwrap();
    };

    let sample_text = "This   is a sample text with    multiple spaces,\nlongwordwithoutspaceswhichshouldtestoverflowwrap and hyphen-should-break.";

    // Normal (default)
    let t1 = Text {
        text: sample_text.to_string(),
        color: RGBA::WHITE,
        layout: TextLayout {
            white_space: WhiteSpace::Normal,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "white-space: Normal", t1);

    // NoWrap
    let t2 = Text {
        text: sample_text.to_string(),
        color: RGBA::LIGHTBLUE,
        layout: TextLayout {
            white_space: WhiteSpace::NoWrap,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "white-space: NoWrap", t2);

    // Pre
    let t3 = Text {
        text: sample_text.to_string(),
        color: RGBA::RED,
        layout: TextLayout {
            white_space: WhiteSpace::Pre,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "white-space: Pre", t3);

    // PreWrap
    let t4 = Text {
        text: sample_text.to_string(),
        color: RGBA::GREEN,
        layout: TextLayout {
            white_space: WhiteSpace::PreWrap,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "white-space: PreWrap", t4);

    // PreLine
    let t5 = Text {
        text: sample_text.to_string(),
        color: RGBA::PINK,
        layout: TextLayout {
            white_space: WhiteSpace::PreLine,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "white-space: PreLine", t5);

    // BreakSpaces
    let t6 = Text {
        text: sample_text.to_string(),
        color: RGBA::PURPLE,
        layout: TextLayout {
            white_space: WhiteSpace::BreakSpaces,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "white-space: BreakSpaces", t6);

    // overflow_wrap: BreakWord demo
    let t7 = Text {
        text: "averyverylongwordthatexceedsthecontainerwidthandshouldbreak".to_string(),
        layout: TextLayout {
            overflow_wrap: OverflowWrap::BreakWord,
            white_space: WhiteSpace::Normal,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "overflow-wrap: BreakWord", t7);

    // TextOverflow: Ellipsis
    let t8 = Text {
        text: "This line will be ellipsized if too long to fit in container".to_string(),
        layout: TextLayout {
            overflow: TextOverflow::Ellipsis,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "text-overflow: Ellipsis", t8);

    // TextOverflow: Clip
    let t9 = Text {
        text: "This line will be clipped if too long to fit in container".to_string(),
        layout: TextLayout {
            overflow: TextOverflow::Clip,
            ..Default::default()
        },
        ..Default::default()
    };
    add_sample(&mut ui, scroll, "text-overflow: Clip", t9);

    drop(ui);
    TestApp::run(global_ui)
}
