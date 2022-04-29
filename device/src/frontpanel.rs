
/// A button which have tre state (suggestion off, short-press, and long-press).
/// A force action is meant to signify some action with significant consequences 
/// so that users cannot accidentelly activate it,
/// such as resetting LAN settings or forcing a device to local control.
pub enum TristateButtonState {
    // Not activated
    Clear,
    // Activated
    Set,
    // Forcefully activated
    Force
}

pub enum LanStateLed {
    Normal,
    Identify,
    Fault
}


/// Frontpanel controls for a device. 
pub trait FrontPanel {
    type Error;

    /// Indicates if a "LAN configuration initialize" (LCI) or "LAN reset" button is pressed.
    /// 
    /// Required by LXI device specification.
    fn lan_reset_pressed(&mut self) -> Result<TristateButtonState, Self::Error>;

    ///  Indicates if a "Local" button is pressed
    fn local_pressed(&mut self) -> Result<TristateButtonState, Self::Error> {
        return Ok(TristateButtonState::Clear)
    }

    /// Control LAN status LED.
    /// 
    /// Might be controlled by hardware, in which case this method does nothing.
    fn set_lan_status_led(&mut self, state: LanStateLed) -> Result<(), Self::Error> {
        // Do nothing
        Ok(())
    }

}