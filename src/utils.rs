use std::fmt::{Display, Formatter};

/// A way to show something, not an alternate placeholder.
pub struct OrNone<'a, T>(pub &'a Option<T>)
where
    T: Display;

impl<'a, T> Display for OrNone<'a, T>
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
