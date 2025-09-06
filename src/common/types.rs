pub trait HasPosition {
    fn get_position(&self) -> &[f32; 3];
}
