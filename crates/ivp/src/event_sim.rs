// The step a controller's simulation runs in (`IvpEventSim`, 1.5): the step and its inverse, and the sim unit the
// controller acts for. Shared by the friction and constraint controllers.

#[derive(Clone, Copy, Debug, Default)]
pub struct EventSim {
    pub delta_time: f64,
    pub i_delta_time: f64,
    /// The sim unit whose cores the controller acts on.
    pub sim_unit: usize,
}
