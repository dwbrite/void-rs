use winit_input_helper::WinitInputHelper;

pub struct Controls {
    pub(crate) input_helper: WinitInputHelper,
}

impl Controls {}

impl Default for Controls {
    fn default() -> Self {
        Self {
            input_helper: WinitInputHelper::new(),
        }
    }
}
