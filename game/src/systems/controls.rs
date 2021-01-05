use winit::event::VirtualKeyCode;

pub struct Controls {
    pub enter: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub shift: bool,
    pub caps: bool
}

impl Controls {
    pub fn key_pressed(&mut self, keycode: VirtualKeyCode) {
        match keycode {
            VirtualKeyCode::Up => { self.up = true; self.down = false; }
            VirtualKeyCode::Down => { self.down = true; self.up = false; }
            VirtualKeyCode::Left => { self.left = true; self.right = false; }
            VirtualKeyCode::Right => { self.right = true; self.left = false; }
            VirtualKeyCode::Return => { self.enter = true; }
            VirtualKeyCode::LShift => { self.shift = true; }
            VirtualKeyCode::RShift => { self.shift = true; }
            VirtualKeyCode::Capital => { self.caps = !self.caps; } // hmmmm
            _ => {}
        }
    }

    pub fn key_released(&mut self, keycode: VirtualKeyCode) {
        match keycode {
            VirtualKeyCode::Up => { self.up = false; }
            VirtualKeyCode::Down => { self.down = false; }
            VirtualKeyCode::Left => { self.left = false; }
            VirtualKeyCode::Right => { self.right = false; }
            VirtualKeyCode::Return => { self.enter = false; }
            VirtualKeyCode::LShift => { self.shift = false; }
            VirtualKeyCode::RShift => { self.shift = false; }
            _ => {}
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            //input_char: String::new(),
            enter: false,
            up: false,
            down: false,
            left: false,
            right: false,
            shift: false,
            caps: false
        }
    }
}