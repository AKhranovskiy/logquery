use crossterm::event::KeyEvent;

pub trait KeyEventHandler {
    type Action;
    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<Self::Action> {
        let _ = event;
        None
    }
}
