pub trait Take<T> {
    fn take(&mut self) -> T;
}


impl Take<bool> for bool {
    fn take(&mut self) -> bool {
        let ret_val = *self;

        *self = false;

        ret_val
    }
}
