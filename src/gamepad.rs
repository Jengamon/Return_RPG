///! Virtual gamepad simulation.
///! We go for the basic of basic controllers: an Xbox-style simulation

#[derive(Debug, Clone, Copy, Default)]
pub struct VirtualGamepadState {
    pub l_x_axis: f32,
    pub l_y_axis: f32,
    pub r_x_axis: f32,
    pub r_y_axis: f32,
    pub l_trigger: f32,
    pub r_trigger: f32,

    pub a_button: bool,
    pub b_button: bool,
    pub x_button: bool,
    pub y_button: bool,
    pub start_button: bool,
    pub select_button: bool,
    pub r_bumper: bool,
    pub l_bumper: bool
}

impl VirtualGamepadState {
    pub fn new() -> VirtualGamepadState {
        Default::default()
    }
}