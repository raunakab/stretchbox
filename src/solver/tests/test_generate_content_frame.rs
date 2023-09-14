use crate::{Padding, solver::generate_content_frame, Frame};

#[test]
fn test_generate_content_frame_with_zero_length_x_and_no_padding() {
    let length_x = 0.;
    let padding = Padding::default();

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 0., length_x: 0. };

    assert_eq!(actual_content_frame, expected_content_frame);
}

#[test]
fn test_generate_content_frame_with_no_padding() {
    let length_x = 100.;
    let padding = Padding::default();

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 0., length_x: 100. };

    assert_eq!(actual_content_frame, expected_content_frame);
}

#[test]
fn test_generate_content_frame_with_zero_length_x() {
    let length_x = 0.;
    let padding = Padding { start_x: 10., end_x: 10. };

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 0., length_x: 0. };

    assert_eq!(actual_content_frame, expected_content_frame);
}

#[test]
fn test_generate_content_frame() {
    let length_x = 100.;
    let padding = Padding { start_x: 10., end_x: 10. };

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 10., length_x: 80. };

    assert_eq!(actual_content_frame, expected_content_frame);
}

#[test]
fn test_generate_content_frame_with_padding_start_x_greater_than_length_x() {
    let length_x = 100.;
    let padding = Padding { start_x: 110., end_x: 10. };

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 100., length_x: 0. };

    assert_eq!(actual_content_frame, expected_content_frame);
}

#[test]
fn test_generate_content_frame_with_padding_end_x_greater_than_length_x() {
    let length_x = 100.;
    let padding = Padding { start_x: 10., end_x: 110. };

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 10., length_x: 0. };

    assert_eq!(actual_content_frame, expected_content_frame);
}

#[test]
fn test_generate_content_frame_with_padding_start_x_and_end_x_greater_than_length_x() {
    let length_x = 100.;
    let padding = Padding { start_x: 110., end_x: 110. };

    let actual_content_frame = generate_content_frame(length_x, padding);
    let expected_content_frame = Frame { offset_x: 100., length_x: 0. };

    assert_eq!(actual_content_frame, expected_content_frame);
}
