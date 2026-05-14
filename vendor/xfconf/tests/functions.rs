use xfconf::*;

// ignored because in CI there is no valid setting
#[ignore]
#[test]
fn get_int_should_get_valid_value() {
    init().expect("Failed to initialize xfconf");
    let channel = Channel::get("xsettings");
    let value = channel.get_property::<i32>("/Gdk/WindowScalingFactor");
    assert_eq!(value, None);
    unsafe {
        shutdown();
    }
}
