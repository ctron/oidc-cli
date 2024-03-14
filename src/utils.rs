use std::fmt::{Display, Formatter};

/// A way to show something, not an alternate placeholder.
pub struct OrNone<T>(pub Option<T>)
where
    T: Display;

impl<T> Display for OrNone<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(value) => value.fmt(f),
            None => f.write_str("n/a"),
        }
    }
}
